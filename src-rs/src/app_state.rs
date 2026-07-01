use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use tauri::AppHandle;
use tokio::{
    sync::{oneshot, Mutex, RwLock},
    task::JoinHandle,
};

use crate::{
    discord_rpc::{DiscordRpcManager, RpcConfig},
    riot::{
        cache::PublicCacheContext,
        state::{
            now_timestamp, AppSnapshot, CoreStatus, CoreStatusKind, DiagnosticSnapshot,
            LiveSnapshot, LocalizedMessage, MatchPhase, OverlayStatus, RpcStatus,
        },
        valorant_client::{ValorantContentCache, ValorantPresentation},
        watcher::{run_monitor_loop, PollResult},
    },
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<AppInner>>,
    monitor: Arc<Mutex<Option<MonitorHandle>>>,
    discord: Arc<Mutex<DiscordRpcManager>>,
    overlay_status: Arc<RwLock<OverlayStatus>>,
    overlay_server: Arc<Mutex<Option<JoinHandle<()>>>>,
    content_cache: ValorantContentCache,
    public_cache: Arc<OnceLock<PublicCacheContext>>,
}

struct AppInner {
    status: CoreStatus,
    diagnostics: DiagnosticSnapshot,
    live_snapshot: Option<LiveSnapshot>,
}

struct MonitorHandle {
    stop_tx: oneshot::Sender<()>,
    join: JoinHandle<()>,
}

pub struct AppliedChanges {
    pub status: Option<CoreStatus>,
    pub live_snapshot: Option<Option<LiveSnapshot>>,
    pub rpc_status: Option<RpcStatus>,
}

impl AppState {
    pub fn new() -> Self {
        let status = CoreStatus::new(
            CoreStatusKind::Disconnected,
            false,
            LocalizedMessage::key("status.message.notStarted"),
        );
        let diagnostics = DiagnosticSnapshot::empty(status.clone());

        Self {
            inner: Arc::new(RwLock::new(AppInner {
                status,
                diagnostics,
                live_snapshot: None,
            })),
            monitor: Arc::new(Mutex::new(None)),
            discord: Arc::new(Mutex::new(DiscordRpcManager::new(RpcConfig::from_env()))),
            overlay_status: Arc::new(RwLock::new(OverlayStatus::new(
                false,
                None,
                None,
                LocalizedMessage::key("status.overlay.notStarted"),
            ))),
            overlay_server: Arc::new(Mutex::new(None)),
            content_cache: ValorantContentCache::default(),
            public_cache: Arc::new(OnceLock::new()),
        }
    }

    pub async fn start_overlay_server(&self) -> OverlayStatus {
        let mut server = self.overlay_server.lock().await;
        if server.is_some() {
            return self.overlay_status().await;
        }

        self.set_overlay_status(OverlayStatus::new(
            false,
            None,
            None,
            LocalizedMessage::key("status.overlay.starting"),
        ))
        .await;

        let state = self.clone();
        let join = tokio::spawn(async move {
            crate::overlay::run_overlay_server(state).await;
        });
        *server = Some(join);

        self.overlay_status().await
    }

    pub async fn start_monitor(&self, app: AppHandle) -> CoreStatus {
        let mut monitor = self.monitor.lock().await;
        if monitor.is_some() {
            return self.status().await;
        }

        let starting = CoreStatus::new(
            CoreStatusKind::Disconnected,
            true,
            LocalizedMessage::key("status.message.starting"),
        );
        self.set_status(starting.clone()).await;

        let (stop_tx, stop_rx) = oneshot::channel();
        let state = self.clone();
        let join = tokio::spawn(async move {
            run_monitor_loop(state, app, stop_rx).await;
        });

        *monitor = Some(MonitorHandle { stop_tx, join });
        starting
    }

    pub async fn stop_monitor(&self) -> CoreStatus {
        let handle = self.monitor.lock().await.take();
        if let Some(handle) = handle {
            let _ = handle.stop_tx.send(());
            let _ = handle.join.await;
        }

        let stopped = CoreStatus::new(
            CoreStatusKind::Disconnected,
            false,
            LocalizedMessage::key("status.message.stopped"),
        );
        self.set_status(stopped.clone()).await;
        stopped
    }

    pub async fn status(&self) -> CoreStatus {
        self.inner.read().await.status.clone()
    }

    pub async fn diagnostics(&self) -> DiagnosticSnapshot {
        self.inner.read().await.diagnostics.clone()
    }

    pub async fn live_snapshot(&self) -> Option<LiveSnapshot> {
        self.inner.read().await.live_snapshot.clone()
    }

    pub async fn rpc_status(&self) -> RpcStatus {
        self.discord.lock().await.status()
    }

    pub async fn overlay_status(&self) -> OverlayStatus {
        self.overlay_status.read().await.clone()
    }

    pub async fn app_snapshot(&self) -> AppSnapshot {
        let (diagnostics, live_snapshot, rpc_status, overlay_status) = tokio::join!(
            self.diagnostics(),
            self.live_snapshot(),
            self.rpc_status(),
            self.overlay_status()
        );
        AppSnapshot {
            diagnostics,
            live_snapshot,
            rpc_status,
            overlay_status,
        }
    }

    pub async fn set_overlay_status(&self, status: OverlayStatus) {
        *self.overlay_status.write().await = status;
    }

    pub async fn set_rpc_enabled(&self, enabled: bool) -> RpcStatus {
        let snapshot = self.live_snapshot().await;
        self.discord
            .lock()
            .await
            .set_enabled(enabled, snapshot.as_ref())
    }

    pub async fn set_rpc_locale(&self, locale: String) -> RpcStatus {
        let content = self.content_cache.get(&locale).await.ok();
        let snapshot = self.live_snapshot().await;
        self.discord
            .lock()
            .await
            .set_locale(locale, content, snapshot.as_ref())
    }

    pub async fn valorant_presentation(
        &self,
        locale: &str,
        agent_id: Option<&str>,
        map_id: Option<&str>,
        tier: Option<u32>,
    ) -> Result<ValorantPresentation, String> {
        let presentation = self
            .content_cache
            .get(locale)
            .await
            .map(|content| content.presentation(agent_id, map_id, tier))
            .map_err(|err| err.message)?;
        Ok(self
            .content_cache
            .cache_presentation_assets(presentation)
            .await)
    }

    pub fn content_cache(&self) -> ValorantContentCache {
        self.content_cache.clone()
    }

    pub fn configure_public_cache(&self, root: PathBuf, app_version: String) {
        let context = PublicCacheContext::new(root, app_version);
        self.content_cache.configure_public_cache(context.clone());
        let _ = self.public_cache.set(context);
    }

    pub fn public_cache_context(&self) -> Option<PublicCacheContext> {
        self.public_cache.get().cloned()
    }

    pub async fn apply_poll_result(&self, result: PollResult) -> AppliedChanges {
        let (status_change, snapshot_change, snapshot_for_rpc) = {
            let mut inner = self.inner.write().await;
            let mut live_snapshot = result.live_snapshot;
            if let Some(snapshot) = &mut live_snapshot {
                assign_session_started_at(inner.live_snapshot.as_ref(), snapshot);
            }

            let status_change = if inner.status != result.status {
                Some(result.status.clone())
            } else {
                None
            };
            let snapshot_change = if inner.live_snapshot != live_snapshot {
                Some(live_snapshot.clone())
            } else {
                None
            };

            inner.status = result.status;
            inner.diagnostics = result.diagnostics;
            inner.live_snapshot = live_snapshot.clone();

            (status_change, snapshot_change, live_snapshot)
        };

        let rpc_status = if let Some(snapshot) = snapshot_for_rpc.as_ref() {
            let mut discord = self.discord.lock().await;
            let before = discord.status();
            let after = discord.update_snapshot(snapshot);
            if after != before {
                Some(after)
            } else {
                None
            }
        } else {
            None
        };

        AppliedChanges {
            status: status_change,
            live_snapshot: snapshot_change,
            rpc_status,
        }
    }

    async fn set_status(&self, status: CoreStatus) {
        let mut inner = self.inner.write().await;
        inner.status = status.clone();
        inner.diagnostics.status = status;
        inner.diagnostics.touch();
    }
}

fn assign_session_started_at(previous: Option<&LiveSnapshot>, current: &mut LiveSnapshot) {
    if current.session_started_at.is_some() {
        return;
    }

    if let Some(previous) = previous.filter(|previous| same_live_session(previous, current)) {
        current.session_started_at = previous
            .session_started_at
            .clone()
            .or_else(|| Some(previous.updated_at.clone()));
        return;
    }

    current.session_started_at = Some(now_timestamp());
}

fn same_live_session(previous: &LiveSnapshot, current: &LiveSnapshot) -> bool {
    match (
        previous
            .match_id
            .as_deref()
            .filter(|value| !value.is_empty()),
        current
            .match_id
            .as_deref()
            .filter(|value| !value.is_empty()),
    ) {
        (Some(previous), Some(current)) => return previous == current,
        (Some(_), None) | (None, Some(_)) => return false,
        (None, None) => {}
    }

    previous.phase == current.phase
        && previous.queue_id == current.queue_id
        && match current.phase {
            MatchPhase::Ingame | MatchPhase::Replay | MatchPhase::Range => {
                previous.map_id == current.map_id && previous.agent_id == current.agent_id
            }
            MatchPhase::Pregame => previous.agent_id == current.agent_id,
            MatchPhase::Menus | MatchPhase::Matchmaking | MatchPhase::Unknown => true,
        }
}

#[cfg(test)]
mod tests {
    use crate::riot::state::{
        LiveSnapshot, MatchPhase, PartySnapshot, PlayerIdentity, ScoreSnapshot,
    };

    use super::{assign_session_started_at, AppState};

    fn snapshot(phase: MatchPhase) -> LiveSnapshot {
        LiveSnapshot {
            phase,
            player: PlayerIdentity::default(),
            region: Some("ap".to_string()),
            shard: Some("ap".to_string()),
            queue_id: Some("competitive".to_string()),
            queue_key: Some("competitive".to_string()),
            party: PartySnapshot {
                state: None,
                size: Some(1),
                max_size: Some(5),
                accessibility: None,
            },
            map_id: Some("/Game/Maps/Ascent/Ascent".to_string()),
            map_name: Some("Ascent".to_string()),
            agent_id: Some("add6443a-41bd-e414-f6ad-e58d267f4e95".to_string()),
            agent_name: Some("Jett".to_string()),
            score: Some(ScoreSnapshot { ally: 1, enemy: 0 }),
            rank: None,
            match_id: Some("match-1".to_string()),
            session_started_at: None,
            updated_at: "2026-06-26T10:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn preserves_session_start_for_score_updates_in_same_match() {
        let mut previous = snapshot(MatchPhase::Ingame);
        previous.session_started_at = Some("2026-06-26T09:50:00.000Z".to_string());

        let mut current = snapshot(MatchPhase::Ingame);
        current.score = Some(ScoreSnapshot { ally: 2, enemy: 0 });
        current.updated_at = "2026-06-26T10:01:00.000Z".to_string();

        assign_session_started_at(Some(&previous), &mut current);

        assert_eq!(current.session_started_at, previous.session_started_at);
    }

    #[test]
    fn starts_new_timer_for_new_match() {
        let mut previous = snapshot(MatchPhase::Ingame);
        previous.session_started_at = Some("2026-06-26T09:50:00.000Z".to_string());

        let mut current = snapshot(MatchPhase::Ingame);
        current.match_id = Some("match-2".to_string());

        assign_session_started_at(Some(&previous), &mut current);

        assert_ne!(current.session_started_at, previous.session_started_at);
    }

    #[tokio::test]
    async fn bundles_frontend_state_into_one_snapshot() {
        let state = AppState::new();
        let snapshot = state.app_snapshot().await;

        assert_eq!(
            snapshot.diagnostics.status.kind,
            crate::riot::state::CoreStatusKind::Disconnected
        );
        assert!(snapshot.live_snapshot.is_none());
        assert!(!snapshot.overlay_status.enabled);
    }
}
