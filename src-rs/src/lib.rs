mod app_state;
mod commands;
mod discord_rpc;
mod overlay;
mod riot;

// Missing community translation keys fall back to this English catalog.
rust_i18n::i18n!("locales", fallback = "en-US");

use app_state::AppState;
use rust_i18n::t;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_store::StoreExt;

const SETTINGS_STORE: &str = "settings.json";
const DEFAULT_MINIMIZE_TO_TRAY: bool = true;

#[cfg(not(debug_assertions))]
fn prevent_default_shortcuts() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_prevent_default::{Builder, PlatformOptions};

    Builder::new()
        .platform(
            PlatformOptions::new()
                .browser_accelerator_keys(false)
                .default_context_menus(false),
        )
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    let overlay_state = state.clone();

    let builder = {
        let builder = tauri::Builder::default()
            .manage(state)
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_store::Builder::new().build())
            .plugin(tauri_plugin_autostart::Builder::new().build())
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                show_main_window(app);
            }));

        #[cfg(not(debug_assertions))]
        {
            builder.plugin(prevent_default_shortcuts())
        }

        #[cfg(debug_assertions)]
        {
            builder
        }
    };

    builder
        .setup(move |app| {
            let app_state = app.state::<AppState>();
            app_state.configure_public_cache(
                app.path().app_cache_dir()?,
                app.package_info().version.to_string(),
            );

            let state = overlay_state.clone();
            tauri::async_runtime::spawn(async move {
                state.start_overlay_server().await;
            });

            build_tray(app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if minimize_to_tray_enabled(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_get_snapshot,
            commands::riot_start_monitor,
            commands::riot_stop_monitor,
            commands::riot_get_diagnostics,
            commands::riot_get_live_snapshot,
            commands::valorant_get_presentation,
            commands::discord_rpc_set_enabled,
            commands::discord_rpc_get_status,
            commands::discord_rpc_set_locale,
            commands::localization_set_ui_locale,
            commands::overlay_get_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = localized_tray_menu(app)?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Radianite")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn localized_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", &t!("tray.show"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &t!("tray.quit"), true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

pub(crate) fn refresh_tray_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(localized_tray_menu(app)?))?;
    }
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn minimize_to_tray_enabled(app: &tauri::AppHandle) -> bool {
    app.get_store(SETTINGS_STORE)
        .and_then(|store| store.get("minimizeToTray"))
        .and_then(|value| value.as_bool())
        .unwrap_or(DEFAULT_MINIMIZE_TO_TRAY)
}
