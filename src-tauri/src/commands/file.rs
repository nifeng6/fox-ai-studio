use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDialogResult {
    pub canceled: bool,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDialogResult {
    pub canceled: bool,
    pub file_path: String,
}

#[tauri::command]
pub async fn open_file_dialog(options: Option<serde_json::Value>) -> Result<FileDialogResult, String> {
    log::info!("Open file dialog with options: {:?}", options);
    Ok(FileDialogResult {
        canceled: true,
        file_paths: vec![],
    })
}

#[tauri::command]
pub async fn save_file_dialog(options: Option<serde_json::Value>) -> Result<SaveDialogResult, String> {
    log::info!("Save file dialog with options: {:?}", options);
    Ok(SaveDialogResult {
        canceled: true,
        file_path: String::new(),
    })
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn get_app_data_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
