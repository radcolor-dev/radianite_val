use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedMessage {
    pub key: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl LocalizedMessage {
    pub fn key(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: BTreeMap::new(),
            detail: None,
        }
    }

    pub fn technical(key: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: BTreeMap::new(),
            detail: Some(detail.into()),
        }
    }
}

impl From<&str> for LocalizedMessage {
    fn from(detail: &str) -> Self {
        Self::technical("status.message.technical", detail)
    }
}

impl From<String> for LocalizedMessage {
    fn from(detail: String) -> Self {
        Self::technical("status.message.technical", detail)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CoreStatusKind {
    NoRiotInstall,
    RiotClientClosed,
    RiotClientOnly,
    ValorantLaunching,
    ValorantReady,
    AuthExpired,
    Disconnected,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreStatus {
    pub kind: CoreStatusKind,
    pub message: LocalizedMessage,
    pub monitored: bool,
    pub updated_at: String,
}

impl CoreStatus {
    pub fn new(
        kind: CoreStatusKind,
        monitored: bool,
        message: impl Into<LocalizedMessage>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            monitored,
            updated_at: now_timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MatchPhase {
    Menus,
    Matchmaking,
    Pregame,
    Ingame,
    Range,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerIdentity {
    pub puuid_present: bool,
    pub game_name: Option<String>,
    pub game_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PartySnapshot {
    pub state: Option<String>,
    pub size: Option<u32>,
    pub max_size: Option<u32>,
    pub accessibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSnapshot {
    pub ally: u32,
    pub enemy: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RankSnapshot {
    pub tier: Option<u32>,
    pub tier_name: Option<String>,
    pub ranked_rating: Option<i32>,
    pub last_match_delta: Option<i32>,
    pub leaderboard_rank: Option<u32>,
    pub season_id: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    pub enabled: bool,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub message: LocalizedMessage,
    pub updated_at: String,
}

impl OverlayStatus {
    pub fn new(
        enabled: bool,
        url: Option<String>,
        port: Option<u16>,
        message: impl Into<LocalizedMessage>,
    ) -> Self {
        Self {
            enabled,
            url,
            port,
            message: message.into(),
            updated_at: now_timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveSnapshot {
    pub phase: MatchPhase,
    pub player: PlayerIdentity,
    pub region: Option<String>,
    pub shard: Option<String>,
    pub queue_id: Option<String>,
    pub party: PartySnapshot,
    pub map_id: Option<String>,
    pub map_name: Option<String>,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub score: Option<ScoreSnapshot>,
    pub rank: Option<RankSnapshot>,
    pub match_id: Option<String>,
    pub session_started_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub diagnostics: DiagnosticSnapshot,
    pub live_snapshot: Option<LiveSnapshot>,
    pub rpc_status: RpcStatus,
    pub overlay_status: OverlayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSnapshot {
    pub status: CoreStatus,
    pub riot_installs_json_exists: bool,
    pub riot_installs_path: Option<String>,
    pub lockfile_exists: bool,
    pub lockfile_path: Option<String>,
    pub lockfile_pid: Option<u32>,
    pub lockfile_protocol: Option<String>,
    pub lockfile_port_present: bool,
    pub local_api_ready: bool,
    pub riot_client_sessions_status: Option<u16>,
    pub session_product_ids: Vec<String>,
    pub valorant_session_present: bool,
    pub region: Option<String>,
    pub shard: Option<String>,
    pub client_version: Option<String>,
    pub puuid_present: bool,
    pub game_name: Option<String>,
    pub game_tag: Option<String>,
    pub access_token_ready: bool,
    pub entitlement_token_ready: bool,
    pub last_error: Option<String>,
    pub updated_at: String,
}

impl DiagnosticSnapshot {
    pub fn empty(status: CoreStatus) -> Self {
        Self {
            status,
            riot_installs_json_exists: false,
            riot_installs_path: None,
            lockfile_exists: false,
            lockfile_path: None,
            lockfile_pid: None,
            lockfile_protocol: None,
            lockfile_port_present: false,
            local_api_ready: false,
            riot_client_sessions_status: None,
            session_product_ids: Vec::new(),
            valorant_session_present: false,
            region: None,
            shard: None,
            client_version: None,
            puuid_present: false,
            game_name: None,
            game_tag: None,
            access_token_ready: false,
            entitlement_token_ready: false,
            last_error: None,
            updated_at: now_timestamp(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now_timestamp();
        self.status.updated_at = self.updated_at.clone();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcStatus {
    pub enabled: bool,
    pub connected: bool,
    pub configured: bool,
    pub message: LocalizedMessage,
    pub locale: String,
    pub preview: Option<RpcPreview>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcPreview {
    pub name: String,
    pub details: String,
    pub state: String,
    pub started_at: Option<i64>,
}

impl RpcStatus {
    pub fn new(
        enabled: bool,
        connected: bool,
        configured: bool,
        message: impl Into<LocalizedMessage>,
    ) -> Self {
        Self {
            enabled,
            connected,
            configured,
            message: message.into(),
            locale: "en-US".to_string(),
            preview: None,
            updated_at: now_timestamp(),
        }
    }
}
