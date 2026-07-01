use tauri::{AppHandle, Emitter, State};

use crate::{
    app_state::AppState,
    riot::{
        state::{
            AppSnapshot, CoreStatus, DiagnosticSnapshot, LiveSnapshot, OverlayStatus, RpcStatus,
        },
        valorant_client::ValorantPresentation,
    },
    settings::{self, Settings, SettingsBootstrap},
};

#[tauri::command]
pub async fn app_get_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    Ok(state.app_snapshot().await)
}

#[tauri::command]
pub async fn riot_start_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CoreStatus, String> {
    let status = state.start_monitor(app.clone()).await;
    app.emit("riot:status", status.clone())
        .map_err(|err| err.to_string())?;
    Ok(status)
}

#[tauri::command]
pub async fn riot_stop_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CoreStatus, String> {
    let status = state.stop_monitor().await;
    app.emit("riot:status", status.clone())
        .map_err(|err| err.to_string())?;
    Ok(status)
}

#[tauri::command]
pub async fn riot_get_diagnostics(
    state: State<'_, AppState>,
) -> Result<DiagnosticSnapshot, String> {
    Ok(state.diagnostics().await)
}

#[tauri::command]
pub async fn riot_get_live_snapshot(
    state: State<'_, AppState>,
) -> Result<Option<LiveSnapshot>, String> {
    Ok(state.live_snapshot().await)
}

#[tauri::command]
pub async fn valorant_get_presentation(
    state: State<'_, AppState>,
    locale: String,
    agent_id: Option<String>,
    map_id: Option<String>,
    tier: Option<u32>,
) -> Result<ValorantPresentation, String> {
    if !rust_i18n::available_locales!()
        .iter()
        .any(|available| *available == locale)
    {
        return Err(format!("unsupported UI locale: {locale}"));
    }
    state
        .valorant_presentation(&locale, agent_id.as_deref(), map_id.as_deref(), tier)
        .await
}

#[tauri::command]
pub async fn discord_rpc_set_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<RpcStatus, String> {
    let status = state.set_rpc_enabled(enabled).await;
    app.emit("discord:status", status.clone())
        .map_err(|err| err.to_string())?;
    Ok(status)
}

#[tauri::command]
pub async fn discord_rpc_get_status(state: State<'_, AppState>) -> Result<RpcStatus, String> {
    Ok(state.rpc_status().await)
}

#[tauri::command]
pub async fn discord_rpc_set_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    locale: String,
) -> Result<RpcStatus, String> {
    if !rust_i18n::available_locales!()
        .iter()
        .any(|available| *available == locale)
    {
        return Err(format!("unsupported RPC locale: {locale}"));
    }
    let status = state.set_rpc_locale(locale).await;
    app.emit("discord:status", status.clone())
        .map_err(|err| err.to_string())?;
    Ok(status)
}

#[tauri::command]
pub fn localization_set_ui_locale(app: AppHandle, locale: String) -> Result<(), String> {
    if !rust_i18n::available_locales!()
        .iter()
        .any(|available| *available == locale)
    {
        return Err(format!("unsupported UI locale: {locale}"));
    }
    rust_i18n::set_locale(&locale);
    crate::refresh_tray_menu(&app).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn overlay_get_status(state: State<'_, AppState>) -> Result<OverlayStatus, String> {
    Ok(state.overlay_status().await)
}

#[tauri::command]
pub async fn settings_initialize(
    app: AppHandle,
    state: State<'_, AppState>,
    default_ui_locale: String,
    default_rpc_locale: String,
) -> Result<SettingsBootstrap, String> {
    settings::initialize(&app, &state, default_ui_locale, default_rpc_locale).await
}

#[tauri::command]
pub async fn settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SettingsBootstrap, String> {
    settings::update(&app, &state, settings).await
}
