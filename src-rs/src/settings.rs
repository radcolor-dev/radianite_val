use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;

use crate::{app_state::AppState, riot::state::RpcStatus};

pub const SETTINGS_STORE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub run_at_boot: bool,
    pub minimize_to_tray: bool,
    pub enable_rpc_on_start: bool,
    pub ui_locale: String,
    pub rpc_locale: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBootstrap {
    pub settings: Settings,
    pub rpc_status: RpcStatus,
}

pub async fn initialize(
    app: &AppHandle,
    state: &AppState,
    default_ui_locale: String,
    default_rpc_locale: String,
) -> Result<SettingsBootstrap, String> {
    let store = app.store(SETTINGS_STORE).map_err(|err| err.to_string())?;
    let stored_run_at_boot = store.get("runAtBoot").and_then(|value| value.as_bool());
    let run_at_boot = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(stored_run_at_boot.unwrap_or(false));
    let settings = Settings {
        run_at_boot,
        minimize_to_tray: store
            .get("minimizeToTray")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        enable_rpc_on_start: store
            .get("enableRpcOnStart")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        ui_locale: valid_locale(
            store.get("uiLocale").and_then(json_string),
            &default_ui_locale,
        ),
        rpc_locale: valid_locale(
            store.get("rpcLocale").and_then(json_string),
            &default_rpc_locale,
        ),
    };

    apply_autostart(app, settings.run_at_boot)?;
    apply_ui_locale(app, &settings.ui_locale)?;
    state.set_rpc_locale(settings.rpc_locale.clone()).await;
    let rpc_status = state.set_rpc_enabled(settings.enable_rpc_on_start).await;
    save(app, &settings)?;

    Ok(SettingsBootstrap {
        settings,
        rpc_status,
    })
}

pub async fn update(
    app: &AppHandle,
    state: &AppState,
    settings: Settings,
) -> Result<SettingsBootstrap, String> {
    ensure_locale(&settings.ui_locale)?;
    ensure_locale(&settings.rpc_locale)?;

    let store = app.store(SETTINGS_STORE).map_err(|err| err.to_string())?;
    let previous_ui_locale = store.get("uiLocale").and_then(json_string);
    let previous_rpc_locale = store.get("rpcLocale").and_then(json_string);

    apply_autostart(app, settings.run_at_boot)?;
    if previous_ui_locale.as_deref() != Some(settings.ui_locale.as_str()) {
        apply_ui_locale(app, &settings.ui_locale)?;
    }
    let rpc_status = if previous_rpc_locale.as_deref() != Some(settings.rpc_locale.as_str()) {
        state.set_rpc_locale(settings.rpc_locale.clone()).await
    } else {
        state.rpc_status().await
    };
    save(app, &settings)?;

    Ok(SettingsBootstrap {
        settings,
        rpc_status,
    })
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let autolaunch_enabled = autolaunch.is_enabled().map_err(|err| err.to_string())?;
    if enabled && !autolaunch_enabled {
        autolaunch.enable().map_err(|err| err.to_string())?;
    } else if !enabled && autolaunch_enabled {
        autolaunch.disable().map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn apply_ui_locale(app: &AppHandle, locale: &str) -> Result<(), String> {
    rust_i18n::set_locale(locale);
    crate::refresh_tray_menu(app).map_err(|err| err.to_string())?;
    Ok(())
}

fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|err| err.to_string())?;
    store.set("runAtBoot", settings.run_at_boot);
    store.set("minimizeToTray", settings.minimize_to_tray);
    store.set("enableRpcOnStart", settings.enable_rpc_on_start);
    store.set("uiLocale", settings.ui_locale.clone());
    store.set("rpcLocale", settings.rpc_locale.clone());
    store.save().map_err(|err| err.to_string())?;
    Ok(())
}

fn json_string(value: serde_json::Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned)
}

fn valid_locale(stored: Option<String>, fallback: &str) -> String {
    stored
        .filter(|locale| ensure_locale(locale).is_ok())
        .unwrap_or_else(|| fallback.to_string())
}

fn ensure_locale(locale: &str) -> Result<(), String> {
    if rust_i18n::available_locales!()
        .iter()
        .any(|available| *available == locale)
    {
        Ok(())
    } else {
        Err(format!("unsupported locale: {locale}"))
    }
}
