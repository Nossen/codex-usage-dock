mod app_server;
mod usage;
mod window_tracker;

use tauri::{AppHandle, Manager, State};
use usage::{SharedUsageState, UsageSnapshot};

#[tauri::command]
async fn get_usage_snapshot(state: State<'_, SharedUsageState>) -> Result<UsageSnapshot, String> {
    Ok(state.0.read().await.clone())
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            |_app, _arguments, _working_directory| {},
        ))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .app_name("Codex Usage Dock")
                .build(),
        )
        .setup(|app| {
            let usage_state = SharedUsageState::default();
            app.manage(usage_state.clone());
            app_server::spawn(app.handle().clone(), usage_state);
            window_tracker::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_usage_snapshot, exit_app])
        .run(tauri::generate_context!())
        .expect("error while running Codex Usage Dock");
}
