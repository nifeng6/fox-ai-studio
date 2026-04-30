mod commands;
mod proxy;

use commands::provider::ProviderState;
use commands::knowledge::KnowledgeState;
use commands::mcp::McpState;
use commands::proxy::{ApiServerHandle, ApiServerState};
use commands::agent_loop::AgentLoopState;
use commands::task_manager::TaskManagerState;
use commands::platform_gateway::{GatewayHandle, GatewayState, PlatformConfigsState};
use commands::agent_reflection::ReflectionStateManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, CheckMenuItemBuilder},
    tray::TrayIconBuilder,
    Manager, Emitter, State,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let show_item = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
            let selection_item = CheckMenuItemBuilder::with_id("selection", "划词助手")
                .checked(true)
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&selection_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let icon_bytes = include_bytes!("../icons/32x32.png");
            let icon_img = image::load_from_memory(icon_bytes).expect("decode tray icon");
            let rgba = icon_img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let tray_icon = Image::new_owned(rgba.into_raw(), w, h);

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("Fox AI")
                .menu(&menu)
                .on_menu_event(move |app_handle, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app_handle.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = w.unminimize();
                            }
                        }
                        "selection" => {
                            let _ = app_handle.emit("tray:toggle-selection", ());
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                            let _ = w.unminimize();
                        }
                    }
                })
                .build(app)?;

            // Load persisted providers from disk
            {
                let saved = commands::provider::load_from_disk(app.handle());
                let state: State<ProviderState> = app.state();
                let mut providers = state.0.lock().expect("provider lock");
                *providers = saved;
            }

            // Set task manager app data dir for persistence
            {
                let task_state: State<TaskManagerState> = app.state();
                if let Ok(data_dir) = app.path().app_data_dir() {
                    task_state.set_app_data_dir(data_dir);
                }
            }

            // 崩溃恢复：加载未完成的任务
            {
                let task_state: State<TaskManagerState> = app.state();
                match commands::task_manager::recover_incomplete_tasks(&*task_state) {
                    Ok(count) => {
                        if count > 0 {
                            log::info!("[lib] 恢复了 {} 个未完成任务", count);
                        }
                    }
                    Err(e) => log::warn!("[lib] 任务恢复失败: {}", e),
                }
            }

            Ok(())
        })
        .manage(ProviderState(Mutex::new(vec![])))
        .manage(KnowledgeState(Mutex::new(Vec::new())))
        .manage(McpState(Mutex::new(Vec::new())))
        .manage(ApiServerState(Mutex::new(ApiServerHandle {
            shutdown_tx: None,
            port: 23333,
            running: false,
        })))
        .manage(AgentLoopState(Arc::new(Mutex::new(HashMap::new()))))
        .manage(TaskManagerState::new())
        .manage(GatewayState(Arc::new(Mutex::new(GatewayHandle {
            shutdown_tx: None,
            port: 23334,
            running: false,
        }))))
        .manage(PlatformConfigsState(Arc::new(Mutex::new(HashMap::new()))))
        .manage(ReflectionStateManager::new())
        .invoke_handler(tauri::generate_handler![
            // Chat
            commands::chat::send_chat_message,
            commands::chat::execute_chat_tool,
            commands::chat::abort_chat,
            // Provider
            commands::provider::get_providers,
            commands::provider::add_provider,
            commands::provider::update_provider,
            commands::provider::remove_provider,
            commands::provider::test_connection,
            commands::provider::get_models,
            // Knowledge
            commands::knowledge::get_knowledge_bases,
            commands::knowledge::create_knowledge_base,
            commands::knowledge::delete_knowledge_base,
            commands::knowledge::add_knowledge_document,
            commands::knowledge::search_knowledge,
            // MCP
            commands::mcp::get_mcp_servers,
            commands::mcp::add_mcp_server,
            commands::mcp::remove_mcp_server,
            commands::mcp::start_mcp_server,
            commands::mcp::stop_mcp_server,
            commands::mcp::get_mcp_tools,
            commands::mcp::call_mcp_tool,
            // File
            commands::file::open_file_dialog,
            commands::file::save_file_dialog,
            commands::file::read_file,
            commands::file::write_file,
            commands::file::get_app_data_path,
            // Clipboard Watcher
            commands::clipboard::start_clipboard_watcher,
            commands::clipboard::stop_clipboard_watcher,
            // Selection Watcher (global text selection detection)
            commands::selection::start_selection_watcher,
            commands::selection::stop_selection_watcher,
            // API Proxy Server
            commands::proxy::start_api_server,
            commands::proxy::stop_api_server,
            commands::proxy::get_api_server_status,
            commands::proxy::update_api_server_providers,
            // Desktop (Screenshots)
            commands::desktop::capture_screen,
            commands::desktop::capture_window,
            commands::desktop::list_windows,
            commands::desktop::get_screen_size,
            commands::desktop::get_cursor_position,
            commands::desktop::read_file_base64,
            commands::desktop::write_debug_log,
            commands::desktop::get_screen_elements,
            // Input Control
            commands::input::mouse_move,
            commands::input::mouse_click,
            commands::input::mouse_double_click,
            commands::input::mouse_drag,
            commands::input::mouse_scroll,
            commands::input::keyboard_type,
            commands::input::keyboard_key,
            commands::input::keyboard_hotkey,
            commands::input::action_sequence,
            commands::input::debug_coordinate_info,
            // Computer Use Agent Loop
            commands::agent_loop::start_computer_use,
            commands::agent_loop::stop_computer_use,
            commands::agent_loop::get_computer_use_status,
            commands::agent_loop::approve_action,
            // Channel Notifications
            commands::notification::send_channel_notification,
            commands::notification::test_channel_webhook,
            // Task Manager
            commands::task_manager::create_task,
            commands::task_manager::resume_task,
            commands::task_manager::pause_task,
            commands::task_manager::list_tasks,
            commands::task_manager::get_task_detail,
            commands::task_manager::update_task_plan,
            commands::task_manager::update_task_status,
            commands::task_manager::delete_task,
            commands::task_manager::add_screenshot_history,
            commands::task_manager::add_conversation_entry,
            // Platform Gateway
            commands::platform_gateway::start_platform_gateway,
            commands::platform_gateway::stop_platform_gateway,
            commands::platform_gateway::configure_platform,
            commands::platform_gateway::test_platform,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
