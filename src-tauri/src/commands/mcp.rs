use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub status: String,
    pub tools: Vec<McpTool>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct McpState(pub Mutex<Vec<McpServer>>);

#[tauri::command]
pub fn get_mcp_servers(state: State<'_, McpState>) -> Result<Vec<McpServer>, String> {
    let servers = state.0.lock().map_err(|e| e.to_string())?;
    Ok(servers.clone())
}

#[tauri::command]
pub fn add_mcp_server(server: serde_json::Value, state: State<'_, McpState>) -> Result<McpServer, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let args_val = server.get("args").and_then(|v| v.as_str()).unwrap_or("");
    let args: Vec<String> = if args_val.is_empty() {
        vec![]
    } else {
        args_val.split_whitespace().map(String::from).collect()
    };

    let entry = McpServer {
        id,
        name: server.get("name").and_then(|v| v.as_str()).unwrap_or("新服务").to_string(),
        command: server.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        args,
        enabled: true,
        status: "stopped".to_string(),
        tools: vec![],
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    let mut servers = state.0.lock().map_err(|e| e.to_string())?;
    servers.push(entry.clone());
    Ok(entry)
}

#[tauri::command]
pub fn remove_mcp_server(id: String, state: State<'_, McpState>) -> Result<(), String> {
    let mut servers = state.0.lock().map_err(|e| e.to_string())?;
    servers.retain(|s| s.id != id);
    Ok(())
}

#[tauri::command]
pub fn start_mcp_server(id: String, state: State<'_, McpState>) -> Result<serde_json::Value, String> {
    let mut servers = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = servers.iter_mut().find(|s| s.id == id) {
        s.status = "running".to_string();
        Ok(serde_json::json!({ "success": true, "message": "Server started" }))
    } else {
        Err("Server not found".to_string())
    }
}

#[tauri::command]
pub fn stop_mcp_server(id: String, state: State<'_, McpState>) -> Result<(), String> {
    let mut servers = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = servers.iter_mut().find(|s| s.id == id) {
        s.status = "stopped".to_string();
    }
    Ok(())
}

#[tauri::command]
pub fn get_mcp_tools(id: String, state: State<'_, McpState>) -> Result<Vec<McpTool>, String> {
    let servers = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = servers.iter().find(|s| s.id == id) {
        Ok(s.tools.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn call_mcp_tool(server_id: String, tool_name: String, args: serde_json::Value) -> Result<serde_json::Value, String> {
    log::info!("Call MCP tool {}/{}: {:?}", server_id, tool_name, args);
    Ok(serde_json::json!({ "result": format!("Tool {} executed", tool_name) }))
}
