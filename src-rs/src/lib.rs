mod app_state;
mod commands;
mod discord_rpc;
mod overlay;
mod riot;

use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    let overlay_state = state.clone();

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |_app| {
            let state = overlay_state.clone();
            tauri::async_runtime::spawn(async move {
                state.start_overlay_server().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::riot_start_monitor,
            commands::riot_stop_monitor,
            commands::riot_get_diagnostics,
            commands::riot_get_live_snapshot,
            commands::discord_rpc_set_enabled,
            commands::discord_rpc_get_status,
            commands::overlay_get_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
