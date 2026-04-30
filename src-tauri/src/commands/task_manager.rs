use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

// ── Task State Machine ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    Queued,
    Planning,
    Executing,
    Paused,
    Completed,
    Failed,
}

impl TaskState {
    fn can_transition_to(&self, target: &TaskState) -> bool {
        matches!(
            (self, target),
            (TaskState::Queued, TaskState::Planning)
                | (TaskState::Planning, TaskState::Executing)
                | (TaskState::Planning, TaskState::Paused)
                | (TaskState::Planning, TaskState::Failed)
                | (TaskState::Executing, TaskState::Paused)
                | (TaskState::Executing, TaskState::Completed)
                | (TaskState::Executing, TaskState::Failed)
                | (TaskState::Paused, TaskState::Planning)
                | (TaskState::Paused, TaskState::Executing)
                | (TaskState::Paused, TaskState::Failed)
                | (TaskState::Failed, TaskState::Queued)
                | (TaskState::Completed, TaskState::Queued)
        )
    }
}

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationEntry {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub id: String,
    pub goal: String,
    pub state: TaskState,
    pub plan: Vec<SubTask>,
    pub current_step: u32,
    pub max_steps: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub screenshot_count: u32,
    pub conversation_count: u32,
    pub error_message: Option<String>,
    pub source_platform: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub snapshot: TaskSnapshot,
    pub recent_screenshots: Vec<String>,
    pub recent_conversation: Vec<ConversationEntry>,
}

// ── Internal Full Task ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FullTask {
    snapshot: TaskSnapshot,
    screenshot_history: Vec<String>,
    conversation_history: Vec<ConversationEntry>,
}

impl FullTask {
    fn trimmed_screenshots(&self) -> Vec<String> {
        let start = if self.screenshot_history.len() > 50 {
            self.screenshot_history.len() - 50
        } else {
            0
        };
        self.screenshot_history[start..].to_vec()
    }

    fn trimmed_conversation(&self) -> Vec<ConversationEntry> {
        let start = if self.conversation_history.len() > 200 {
            self.conversation_history.len() - 200
        } else {
            0
        };
        self.conversation_history[start..].to_vec()
    }
}

// ── State ──

pub struct TaskManagerState {
    tasks: Arc<Mutex<HashMap<String, FullTask>>>,
    app_data_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl Clone for TaskManagerState {
    fn clone(&self) -> Self {
        TaskManagerState {
            tasks: Arc::clone(&self.tasks),
            app_data_dir: Arc::clone(&self.app_data_dir),
        }
    }
}

impl TaskManagerState {
    pub fn new() -> Self {
        TaskManagerState {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            app_data_dir: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_data_dir(&self, dir: PathBuf) {
        let mut d = self.app_data_dir.lock().unwrap();
        *d = Some(dir);
    }

    fn get_tasks_dir(&self) -> Result<PathBuf, String> {
        let d = self.app_data_dir.lock().unwrap();
        let base = d.clone().ok_or_else(|| "App data dir not set".to_string())?;
        let tasks_dir = base.join("tasks");
        if !tasks_dir.exists() {
            fs::create_dir_all(&tasks_dir).map_err(|e| format!("Create tasks dir: {}", e))?;
        }
        Ok(tasks_dir)
    }
}

// ── Persistence ──

fn save_task_to_disk(task: &FullTask, tasks_dir: &PathBuf) -> Result<(), String> {
    let path = tasks_dir.join(format!("{}.json", task.snapshot.id));
    let json = serde_json::to_string_pretty(task).map_err(|e| format!("Serialize: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Write: {}", e))
}

fn load_task_from_disk(path: &PathBuf) -> Result<FullTask, String> {
    let json = fs::read_to_string(path).map_err(|e| format!("Read {}: {}", path.display(), e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse {}: {}", path.display(), e))
}

fn load_all_tasks_from_disk(tasks_dir: &PathBuf) -> Vec<FullTask> {
    let mut tasks = Vec::new();
    if let Ok(entries) = fs::read_dir(tasks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(task) = load_task_from_disk(&path) {
                    tasks.push(task);
                }
            }
        }
    }
    tasks
}

// ── Crash Recovery ──

pub fn recover_incomplete_tasks(state: &TaskManagerState) -> Result<u32, String> {
    let tasks_dir = state.get_tasks_dir()?;
    let disk_tasks = load_all_tasks_from_disk(&tasks_dir);
    let mut recovered = 0u32;

    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    for mut task in disk_tasks {
        // Reset in-progress tasks to Paused for crash recovery
        if task.snapshot.state == TaskState::Executing || task.snapshot.state == TaskState::Planning {
            task.snapshot.state = TaskState::Paused;
            task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();
            if let Err(e) = save_task_to_disk(&task, &tasks_dir) {
                log::warn!("[task_manager] Failed to save recovered task {}: {}", task.snapshot.id, e);
            }
            recovered += 1;
        }
        tasks.insert(task.snapshot.id.clone(), task);
    }

    if recovered > 0 {
        log::info!("[task_manager] Recovered {} incomplete tasks", recovered);
    }
    Ok(recovered)
}

// ── Tauri Commands ──

#[tauri::command]
pub fn create_task(
    app: AppHandle,
    goal: String,
    max_steps: Option<u32>,
    source_platform: Option<String>,
    state: State<'_, TaskManagerState>,
) -> Result<TaskSnapshot, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    let snapshot = TaskSnapshot {
        id: id.clone(),
        goal: goal.clone(),
        state: TaskState::Queued,
        plan: Vec::new(),
        current_step: 0,
        max_steps: max_steps.unwrap_or(200),
        created_at: now,
        updated_at: now,
        screenshot_count: 0,
        conversation_count: 0,
        error_message: None,
        source_platform,
        metadata: HashMap::new(),
    };

    let task = FullTask {
        snapshot: snapshot.clone(),
        screenshot_history: Vec::new(),
        conversation_history: Vec::new(),
    };

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(&task, &tasks_dir)?;

    {
        let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
        tasks.insert(id.clone(), task);
    }

    let _ = app.emit("task:created", &snapshot);
    log::info!("[task_manager] Created task {}: {}", id, goal);
    Ok(snapshot)
}

#[tauri::command]
pub fn resume_task(
    app: AppHandle,
    task_id: String,
    state: State<'_, TaskManagerState>,
) -> Result<TaskSnapshot, String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    let target = TaskState::Executing;
    if !task.snapshot.state.can_transition_to(&target) {
        return Err(format!("Cannot resume from {:?} to {:?}", task.snapshot.state, target));
    }

    task.snapshot.state = target.clone();
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;

    let snapshot = task.snapshot.clone();
    drop(tasks);

    let _ = app.emit("task:resumed", &snapshot);
    log::info!("[task_manager] Resumed task {}", task_id);
    Ok(snapshot)
}

#[tauri::command]
pub fn pause_task(
    app: AppHandle,
    task_id: String,
    state: State<'_, TaskManagerState>,
) -> Result<TaskSnapshot, String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    let target = TaskState::Paused;
    if !task.snapshot.state.can_transition_to(&target) {
        return Err(format!("Cannot pause from {:?} to {:?}", task.snapshot.state, target));
    }

    task.snapshot.state = target.clone();
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;

    let snapshot = task.snapshot.clone();
    drop(tasks);

    let _ = app.emit("task:paused", &snapshot);
    log::info!("[task_manager] Paused task {}", task_id);
    Ok(snapshot)
}

#[tauri::command]
pub fn list_tasks(
    state: State<'_, TaskManagerState>,
) -> Result<Vec<TaskSnapshot>, String> {
    // Lazy load from disk if memory is empty
    {
        let tasks = state.tasks.lock().map_err(|e| e.to_string())?;
        if !tasks.is_empty() {
            return Ok(tasks.values().map(|t| t.snapshot.clone()).collect());
        }
    }

    let tasks_dir = state.get_tasks_dir()?;
    let disk_tasks = load_all_tasks_from_disk(&tasks_dir);
    let snapshots: Vec<TaskSnapshot> = disk_tasks.iter().map(|t| t.snapshot.clone()).collect();

    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    for task in disk_tasks {
        tasks.insert(task.snapshot.id.clone(), task);
    }

    Ok(snapshots)
}

#[tauri::command]
pub fn get_task_detail(
    task_id: String,
    state: State<'_, TaskManagerState>,
) -> Result<TaskDetail, String> {
    let tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get(&task_id).ok_or("Task not found")?;

    Ok(TaskDetail {
        snapshot: task.snapshot.clone(),
        recent_screenshots: task.trimmed_screenshots(),
        recent_conversation: task.trimmed_conversation(),
    })
}

#[tauri::command]
pub fn update_task_plan(
    app: AppHandle,
    task_id: String,
    plan: Vec<SubTask>,
    state: State<'_, TaskManagerState>,
) -> Result<TaskSnapshot, String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    task.snapshot.plan = plan;
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    // Auto-transition Queued → Planning when plan is set
    if task.snapshot.state == TaskState::Queued {
        task.snapshot.state = TaskState::Planning;
    }

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;

    let snapshot = task.snapshot.clone();
    drop(tasks);

    let _ = app.emit("task:plan_updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn update_task_status(
    app: AppHandle,
    task_id: String,
    new_state: TaskState,
    error_message: Option<String>,
    state: State<'_, TaskManagerState>,
) -> Result<TaskSnapshot, String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    if !task.snapshot.state.can_transition_to(&new_state) {
        return Err(format!("Invalid transition: {:?} → {:?}", task.snapshot.state, new_state));
    }

    task.snapshot.state = new_state.clone();
    task.snapshot.error_message = error_message;
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;

    let snapshot = task.snapshot.clone();
    drop(tasks);

    let _ = app.emit("task:status_changed", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn delete_task(
    app: AppHandle,
    task_id: String,
    state: State<'_, TaskManagerState>,
) -> Result<(), String> {
    let tasks_dir = state.get_tasks_dir()?;

    {
        let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
        tasks.remove(&task_id);
    }

    let path = tasks_dir.join(format!("{}.json", task_id));
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Delete: {}", e))?;
    }

    let _ = app.emit("task:deleted", task_id);
    log::info!("[task_manager] Deleted task {}", task_id);
    Ok(())
}

#[tauri::command]
pub fn add_screenshot_history(
    task_id: String,
    screenshot_base64: String,
    state: State<'_, TaskManagerState>,
) -> Result<(), String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    // Keep only last 50 screenshots to avoid memory bloat
    if task.screenshot_history.len() >= 50 {
        task.screenshot_history.remove(0);
    }
    task.screenshot_history.push(screenshot_base64);
    task.snapshot.screenshot_count += 1;
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;
    Ok(())
}

#[tauri::command]
pub fn add_conversation_entry(
    task_id: String,
    role: String,
    content: String,
    state: State<'_, TaskManagerState>,
) -> Result<(), String> {
    let mut tasks = state.tasks.lock().map_err(|e| e.to_string())?;
    let task = tasks.get_mut(&task_id).ok_or("Task not found")?;

    let entry = ConversationEntry {
        role,
        content,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    // Keep only last 200 entries
    if task.conversation_history.len() >= 200 {
        task.conversation_history.remove(0);
    }
    task.conversation_history.push(entry);
    task.snapshot.conversation_count += 1;
    task.snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    let tasks_dir = state.get_tasks_dir()?;
    save_task_to_disk(task, &tasks_dir)?;
    Ok(())
}
