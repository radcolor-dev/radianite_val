use std::{
    fs,
    future::Future,
    path::Path,
    pin::Pin,
    time::{Duration, Instant},
};

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::{sync::oneshot, time::sleep};

use crate::app_state::AppState;

use super::{
    local_client::{ChatSession, ExternalSessions, LocalClient, SessionFetch},
    lockfile::{default_paths, LockfilePaths, RiotLockfile},
    state::{
        now_timestamp, CoreStatus, CoreStatusKind, DiagnosticSnapshot, LiveSnapshot,
        LocalizedMessage, MatchPhase, PartySnapshot, PlayerIdentity, RankSnapshot, ScoreSnapshot,
    },
    valorant_client::{
        active_season_id, extract_region_and_shard, fetch_public_client_version, i32_path,
        rank_from_competitive_updates, rank_from_mmr, str_path, u32_path, ValorantClient,
        ValorantContent, ValorantContentCache,
    },
};

const IDENTITY_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub struct PollResult {
    pub status: CoreStatus,
    pub diagnostics: DiagnosticSnapshot,
    pub live_snapshot: Option<LiveSnapshot>,
}

impl PollResult {
    fn next_delay(&self) -> Duration {
        match self.status.kind {
            CoreStatusKind::ValorantReady
            | CoreStatusKind::Degraded
            | CoreStatusKind::AuthExpired => Duration::from_secs(2),
            _ => Duration::from_secs(1),
        }
    }
}

pub trait EventSource {
    fn poll<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = PollResult> + Send + 'a>>;
}

pub struct PollingEventSource {
    paths: LockfilePaths,
    riot_installs_exists: bool,
    last_install_check: Instant,
    cached_lockfile: Option<CachedLockfile>,
    local_client: Option<LocalClient>,
    cached_sessions: Option<CachedSessions>,
    cached_affinity: Option<CachedAffinity>,
    cached_identity: Option<CachedIdentity>,
    cached_rank: Option<RankSnapshot>,
    active_season_id: Option<String>,
    last_rank_fetch: Option<Instant>,
    client_version: Option<String>,
    last_version_attempt: Option<Instant>,
    content: Option<ValorantContent>,
    content_cache: ValorantContentCache,
    last_content_attempt: Option<Instant>,
}

struct CachedLockfile {
    signature: FileSignature,
    value: RiotLockfile,
}

struct CachedSessions {
    fetched_at: Instant,
    value: SessionFetch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedAffinity {
    region: Option<String>,
    shard: Option<String>,
}

struct CachedIdentity {
    fetched_at: Instant,
    value: ChatSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl PollingEventSource {
    pub fn new(content_cache: ValorantContentCache) -> Self {
        let paths = default_paths();
        Self {
            riot_installs_exists: paths
                .riot_installs_path
                .as_ref()
                .is_some_and(|path| path.is_file()),
            paths,
            last_install_check: Instant::now(),
            cached_lockfile: None,
            local_client: None,
            cached_sessions: None,
            cached_affinity: None,
            cached_identity: None,
            cached_rank: None,
            active_season_id: None,
            last_rank_fetch: None,
            client_version: None,
            last_version_attempt: None,
            content: None,
            content_cache,
            last_content_attempt: None,
        }
    }

    async fn poll_once(&mut self) -> PollResult {
        let checking = CoreStatus::new(
            CoreStatusKind::Disconnected,
            true,
            LocalizedMessage::key("status.message.checking"),
        );
        let mut diagnostics = DiagnosticSnapshot::empty(checking);

        diagnostics.riot_installs_path = self
            .paths
            .riot_installs_path
            .as_ref()
            .map(|path| path.display().to_string());
        self.refresh_install_state();
        diagnostics.riot_installs_json_exists = self.riot_installs_exists;

        diagnostics.lockfile_path = self
            .paths
            .lockfile_path
            .as_ref()
            .map(|path| path.display().to_string());
        diagnostics.lockfile_exists = self
            .paths
            .lockfile_path
            .as_ref()
            .is_some_and(|path| path.is_file());

        if !diagnostics.riot_installs_json_exists {
            return finish(
                diagnostics,
                None,
                CoreStatusKind::NoRiotInstall,
                "Riot Client installation metadata was not found",
            );
        }

        if !diagnostics.lockfile_exists {
            return finish(
                diagnostics,
                None,
                CoreStatusKind::RiotClientClosed,
                "Riot Client is not running, or its lockfile is unavailable",
            );
        }

        let lockfile = match self.read_lockfile() {
            Ok(lockfile) => lockfile,
            Err(err) => {
                diagnostics.last_error = Some(err.clone());
                return finish(
                    diagnostics,
                    None,
                    CoreStatusKind::Error,
                    format!("Riot lockfile could not be parsed: {err}"),
                );
            }
        };

        diagnostics.lockfile_pid = Some(lockfile.pid);
        diagnostics.lockfile_protocol = Some(lockfile.protocol.clone());
        diagnostics.lockfile_port_present = lockfile.port > 0;

        let local_client = match self.local_client_for(&lockfile) {
            Ok(client) => client,
            Err(err) => {
                diagnostics.last_error = Some(err.to_string());
                return finish(
                    diagnostics,
                    None,
                    CoreStatusKind::Error,
                    "Riot local HTTP client could not be created",
                );
            }
        };

        let sessions = match self.external_sessions(&local_client).await {
            Ok(fetch) => {
                diagnostics.local_api_ready = true;
                diagnostics.riot_client_sessions_status = Some(fetch.status);
                fetch.sessions
            }
            Err(err) => {
                diagnostics.riot_client_sessions_status = err.status;
                diagnostics.last_error = Some(err.to_string());
                return finish(
                    diagnostics,
                    None,
                    CoreStatusKind::Disconnected,
                    "Riot Client local API is not reachable",
                );
            }
        };

        let mut product_ids = sessions
            .values()
            .filter_map(|session| session.product_id.clone())
            .collect::<Vec<_>>();
        product_ids.sort();
        product_ids.dedup();
        diagnostics.session_product_ids = product_ids;
        diagnostics.valorant_session_present = diagnostics
            .session_product_ids
            .iter()
            .any(|product| product.eq_ignore_ascii_case("valorant"));

        if !diagnostics.valorant_session_present {
            self.cached_affinity = None;
            return finish(
                diagnostics,
                None,
                CoreStatusKind::RiotClientOnly,
                "Riot Client is reachable; VALORANT is not in the active session list",
            );
        }

        let (region, shard) = self.affinity_for(&sessions);
        diagnostics.region = region.clone();
        diagnostics.shard = shard.clone();

        self.refresh_client_version().await;
        self.refresh_content().await;
        diagnostics.client_version = self.client_version.clone();

        let tokens = match local_client.entitlements_token().await {
            Ok(tokens) => {
                diagnostics.access_token_ready = !tokens.access_token.is_empty();
                diagnostics.entitlement_token_ready = !tokens.entitlements_token.is_empty();
                tokens
            }
            Err(err) => {
                diagnostics.last_error = Some(err.to_string());
                let status = if err.status == Some(401) {
                    CoreStatusKind::AuthExpired
                } else {
                    CoreStatusKind::Degraded
                };
                return finish(
                    diagnostics,
                    None,
                    status,
                    "Riot auth token endpoint is unavailable",
                );
            }
        };

        let chat_session = match self.chat_session(&local_client).await {
            Ok(session) => session,
            Err(err) => {
                diagnostics.last_error = Some(err.to_string());
                return finish(
                    diagnostics,
                    None,
                    CoreStatusKind::Degraded,
                    "Riot chat session identity is unavailable",
                );
            }
        };

        let identity = PlayerIdentity {
            puuid_present: chat_session.puuid.is_some(),
            game_name: chat_session.game_name.clone(),
            game_tag: chat_session.game_tag.clone(),
        };
        diagnostics.puuid_present = identity.puuid_present;
        diagnostics.game_name = identity.game_name.clone();
        diagnostics.game_tag = identity.game_tag.clone();

        let Some(puuid) = chat_session.puuid else {
            return finish(
                diagnostics,
                None,
                CoreStatusKind::Degraded,
                "Riot chat session did not include a PUUID",
            );
        };

        let private_presence = match local_client.own_private_presence(&puuid).await {
            Ok(presence) => presence,
            Err(err) => {
                diagnostics.last_error = Some(err.to_string());
                None
            }
        };

        let mut live_snapshot = normalize_live_snapshot(
            private_presence.as_ref(),
            identity,
            region.clone(),
            shard.clone(),
            self.cached_rank.clone(),
        );

        let valorant_client = match (region.clone(), shard.clone()) {
            (Some(region), Some(shard)) => {
                ValorantClient::new(region, shard, tokens, self.client_version.clone()).ok()
            }
            _ => None,
        };

        if let Some(client) = valorant_client.as_ref() {
            self.refresh_rank(client, &puuid).await;
            live_snapshot.rank = self.cached_rank.clone();
            enrich_phase(client, &puuid, &mut live_snapshot).await;
        }
        if let Some(content) = &self.content {
            enrich_content_names(&mut live_snapshot, content);
        }

        let kind = if region.is_none() || shard.is_none() {
            CoreStatusKind::Degraded
        } else if private_presence.is_none() {
            CoreStatusKind::ValorantLaunching
        } else {
            CoreStatusKind::ValorantReady
        };

        let message = match kind {
            CoreStatusKind::Degraded => {
                "VALORANT is active, but region/shard or enrichment data is incomplete"
            }
            CoreStatusKind::ValorantLaunching => {
                "VALORANT is active; waiting for own-player presence data"
            }
            _ => "VALORANT live data is available",
        };

        finish(diagnostics, Some(live_snapshot), kind, message)
    }

    fn refresh_install_state(&mut self) {
        if self.last_install_check.elapsed() < Duration::from_secs(60) {
            return;
        }

        self.last_install_check = Instant::now();
        self.riot_installs_exists = self
            .paths
            .riot_installs_path
            .as_ref()
            .is_some_and(|path| path.is_file());
    }

    fn read_lockfile(&mut self) -> Result<RiotLockfile, String> {
        let path = self
            .paths
            .lockfile_path
            .as_ref()
            .ok_or_else(|| "LOCALAPPDATA is not available".to_string())?
            .clone();
        let signature = file_signature(&path)?;

        if let Some(cached) = self
            .cached_lockfile
            .as_ref()
            .filter(|cached| cached.signature == signature)
        {
            return Ok(cached.value.clone());
        }

        let value = RiotLockfile::read_from_path(path)?;
        self.cached_lockfile = Some(CachedLockfile {
            signature,
            value: value.clone(),
        });
        Ok(value)
    }

    fn local_client_for(&mut self, lockfile: &RiotLockfile) -> Result<LocalClient, String> {
        if self
            .local_client
            .as_ref()
            .is_none_or(|client| !client.matches_lockfile(lockfile))
        {
            self.local_client =
                Some(LocalClient::from_lockfile(lockfile).map_err(|err| err.message)?);
            self.cached_sessions = None;
            self.cached_affinity = None;
            self.cached_identity = None;
        }

        self.local_client
            .clone()
            .ok_or_else(|| "Riot local HTTP client could not be created".to_string())
    }

    async fn external_sessions(
        &mut self,
        local_client: &LocalClient,
    ) -> Result<SessionFetch, super::local_client::LocalClientError> {
        if let Some(cached) = self.cached_sessions.as_ref().filter(|cached| {
            cached.fetched_at.elapsed() < session_cache_ttl(&cached.value.sessions)
        }) {
            return Ok(cached.value.clone());
        }

        let value = local_client.external_sessions().await?;
        self.cached_sessions = Some(CachedSessions {
            fetched_at: Instant::now(),
            value: value.clone(),
        });
        Ok(value)
    }

    fn affinity_for(&mut self, sessions: &ExternalSessions) -> (Option<String>, Option<String>) {
        if let Some(cached) = &self.cached_affinity {
            return (cached.region.clone(), cached.shard.clone());
        }

        let (region, shard) = extract_region_and_shard(sessions);
        self.cached_affinity = Some(CachedAffinity {
            region: region.clone(),
            shard: shard.clone(),
        });
        (region, shard)
    }

    async fn chat_session(
        &mut self,
        local_client: &LocalClient,
    ) -> Result<ChatSession, super::local_client::LocalClientError> {
        if let Some(cached) = self
            .cached_identity
            .as_ref()
            .filter(|cached| cache_is_fresh(cached.fetched_at, Instant::now(), IDENTITY_CACHE_TTL))
        {
            return Ok(cached.value.clone());
        }

        let value = local_client.chat_session().await?;
        self.cached_identity = Some(CachedIdentity {
            fetched_at: Instant::now(),
            value: value.clone(),
        });
        Ok(value)
    }

    async fn refresh_client_version(&mut self) {
        let should_attempt = self.client_version.is_none()
            && self
                .last_version_attempt
                .is_none_or(|attempt| attempt.elapsed() >= Duration::from_secs(300));

        if !should_attempt {
            return;
        }

        self.last_version_attempt = Some(Instant::now());
        if let Ok(version) = fetch_public_client_version().await {
            self.client_version = Some(version);
        }
    }

    async fn refresh_content(&mut self) {
        let should_attempt = self.content.is_none()
            && self
                .last_content_attempt
                .is_none_or(|attempt| attempt.elapsed() >= Duration::from_secs(300));

        if !should_attempt {
            return;
        }

        self.last_content_attempt = Some(Instant::now());
        if let Ok(content) = self.content_cache.get("en-US").await {
            self.content = Some(content);
        }
    }

    async fn refresh_rank(&mut self, client: &ValorantClient, puuid: &str) {
        let should_fetch = self
            .last_rank_fetch
            .is_none_or(|last_fetch| last_fetch.elapsed() >= Duration::from_secs(30));

        if !should_fetch {
            return;
        }

        self.last_rank_fetch = Some(Instant::now());

        if self.active_season_id.is_none() {
            if let Ok(content) = client.content().await {
                self.active_season_id = active_season_id(&content);
            }
        }

        let mut next_rank = client
            .mmr(puuid)
            .await
            .ok()
            .and_then(|mmr| rank_from_mmr(&mmr, self.active_season_id.as_deref()));

        let update_rank = client
            .competitive_updates(puuid)
            .await
            .ok()
            .and_then(|updates| rank_from_competitive_updates(&updates));

        if let Some(update_rank) = update_rank {
            if let Some(rank) = &mut next_rank {
                rank.last_match_delta = update_rank.last_match_delta;
            } else {
                next_rank = Some(update_rank);
            }
        }

        if let Some(rank) = next_rank {
            self.cached_rank = Some(rank);
        }
    }
}

fn session_cache_ttl(sessions: &ExternalSessions) -> Duration {
    if sessions.values().any(|session| {
        session
            .product_id
            .as_deref()
            .is_some_and(|product| product.eq_ignore_ascii_case("valorant"))
    }) {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(3)
    }
}

fn cache_is_fresh(fetched_at: Instant, now: Instant, ttl: Duration) -> bool {
    now.checked_duration_since(fetched_at)
        .is_some_and(|age| age < ttl)
}

fn file_signature(path: &Path) -> Result<FileSignature, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to stat Riot lockfile: {err}"))?;
    Ok(FileSignature {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

impl EventSource for PollingEventSource {
    fn poll<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = PollResult> + Send + 'a>> {
        Box::pin(async move { self.poll_once().await })
    }
}

pub async fn run_monitor_loop(state: AppState, app: AppHandle, mut stop_rx: oneshot::Receiver<()>) {
    let mut source = PollingEventSource::new(state.content_cache());

    loop {
        let result = source.poll().await;
        let delay = result.next_delay();
        let changes = state.apply_poll_result(result).await;

        if let Some(status) = changes.status {
            let _ = app.emit("riot:status", status);
        }
        if let Some(snapshot) = changes.live_snapshot {
            let _ = app.emit("riot:snapshot", snapshot);
        }
        if let Some(rpc_status) = changes.rpc_status {
            let _ = app.emit("discord:status", rpc_status);
        }

        tokio::select! {
            _ = &mut stop_rx => break,
            _ = sleep(delay) => {}
        }
    }
}

fn enrich_content_names(snapshot: &mut LiveSnapshot, content: &ValorantContent) {
    if snapshot.map_name.is_none() {
        snapshot.map_name = snapshot
            .map_id
            .as_deref()
            .and_then(|map_id| content.map_name(map_id));
    }

    if snapshot.agent_name.is_none() {
        snapshot.agent_name = snapshot
            .agent_id
            .as_deref()
            .and_then(|agent_id| content.agent_name(agent_id));
    }

    if let Some(rank) = &mut snapshot.rank {
        if rank.tier_name.is_none() {
            rank.tier_name = rank
                .tier
                .and_then(|tier| content.competitive_tier_name(tier));
        }

        if rank.icon_url.is_none() {
            rank.icon_url = rank
                .tier
                .and_then(|tier| content.competitive_tier_icon_url(tier));
        }
    }
}

fn finish(
    mut diagnostics: DiagnosticSnapshot,
    live_snapshot: Option<LiveSnapshot>,
    kind: CoreStatusKind,
    message: impl Into<String>,
) -> PollResult {
    let detail = message.into();
    let key = match kind {
        CoreStatusKind::NoRiotInstall => "status.message.noInstall",
        CoreStatusKind::RiotClientClosed => "status.message.riotClosed",
        CoreStatusKind::RiotClientOnly => "status.message.riotOnly",
        CoreStatusKind::ValorantLaunching => "status.message.launching",
        CoreStatusKind::ValorantReady => "status.message.ready",
        CoreStatusKind::AuthExpired => "status.message.authExpired",
        CoreStatusKind::Degraded => "status.message.degraded",
        CoreStatusKind::Error => "status.message.error",
        CoreStatusKind::Disconnected => "status.message.technical",
    };
    let message = if matches!(kind, CoreStatusKind::Error) {
        LocalizedMessage::technical(key, detail)
    } else {
        LocalizedMessage::key(key)
    };
    let status = CoreStatus::new(kind, true, message);
    diagnostics.status = status.clone();
    diagnostics.touch();
    PollResult {
        status,
        diagnostics,
        live_snapshot,
    }
}

fn normalize_live_snapshot(
    presence: Option<&Value>,
    player: PlayerIdentity,
    region: Option<String>,
    shard: Option<String>,
    rank: Option<RankSnapshot>,
) -> LiveSnapshot {
    let phase = presence.map(match_phase).unwrap_or(MatchPhase::Menus);
    let queue_id = presence.and_then(|presence| {
        first_str(
            presence,
            &[
                &["partyPresenceData", "queueId"],
                &["matchPresenceData", "queueId"],
                &["queueId"],
            ],
        )
    });

    let party = PartySnapshot {
        state: presence.and_then(|presence| {
            first_str(
                presence,
                &[
                    &["partyPresenceData", "partyState"],
                    &["matchPresenceData", "partyState"],
                    &["partyState"],
                ],
            )
        }),
        size: presence.and_then(|presence| {
            first_u32(
                presence,
                &[
                    &["partyPresenceData", "partySize"],
                    &["matchPresenceData", "partySize"],
                    &["partySize"],
                ],
            )
        }),
        max_size: presence.and_then(|presence| {
            first_u32(
                presence,
                &[
                    &["partyPresenceData", "maxPartySize"],
                    &["matchPresenceData", "maxPartySize"],
                    &["maxPartySize"],
                ],
            )
        }),
        accessibility: presence.and_then(|presence| {
            first_str(
                presence,
                &[
                    &["partyPresenceData", "partyAccessibility"],
                    &["matchPresenceData", "partyAccessibility"],
                    &["partyAccessibility"],
                ],
            )
        }),
    };

    let score = presence.and_then(|presence| {
        let ally = first_u32(
            presence,
            &[
                &["matchPresenceData", "partyOwnerMatchScoreAllyTeam"],
                &["partyOwnerMatchScoreAllyTeam"],
            ],
        )?;
        let enemy = first_u32(
            presence,
            &[
                &["matchPresenceData", "partyOwnerMatchScoreEnemyTeam"],
                &["partyOwnerMatchScoreEnemyTeam"],
            ],
        )?;
        Some(ScoreSnapshot { ally, enemy })
    });

    LiveSnapshot {
        phase,
        player,
        region,
        shard,
        queue_id,
        party,
        map_id: presence.and_then(|presence| {
            first_str(
                presence,
                &[
                    &["matchPresenceData", "mapId"],
                    &["matchPresenceData", "MapID"],
                    &["MapID"],
                ],
            )
        }),
        map_name: None,
        agent_id: presence.and_then(|presence| {
            first_str(
                presence,
                &[
                    &["playerPresenceData", "selectedAgent"],
                    &["playerPresenceData", "characterId"],
                    &["CharacterID"],
                ],
            )
        }),
        agent_name: None,
        score,
        rank,
        match_id: presence.and_then(|presence| {
            first_str(
                presence,
                &[
                    &["matchPresenceData", "matchId"],
                    &["matchPresenceData", "MatchID"],
                    &["MatchID"],
                ],
            )
        }),
        session_started_at: None,
        updated_at: now_timestamp(),
    }
}

fn match_phase(presence: &Value) -> MatchPhase {
    let session = first_str(
        presence,
        &[
            &["matchPresenceData", "sessionLoopState"],
            &["sessionLoopState"],
        ],
    )
    .unwrap_or_default();
    let party_state = first_str(
        presence,
        &[
            &["partyPresenceData", "partyState"],
            &["matchPresenceData", "partyState"],
            &["partyState"],
        ],
    )
    .unwrap_or_default();
    let provisioning_flow = first_str(
        presence,
        &[
            &["matchPresenceData", "provisioningFlow"],
            &["provisioningFlow"],
        ],
    )
    .unwrap_or_default();

    if session == "INGAME" && provisioning_flow == "ShootingRange" {
        return MatchPhase::Range;
    }

    match session.as_str() {
        "MENUS" if party_state == "MATCHMAKING" => MatchPhase::Matchmaking,
        "MENUS" => MatchPhase::Menus,
        "PREGAME" => MatchPhase::Pregame,
        "INGAME" => MatchPhase::Ingame,
        _ => MatchPhase::Unknown,
    }
}

async fn enrich_phase(client: &ValorantClient, puuid: &str, snapshot: &mut LiveSnapshot) {
    match snapshot.phase {
        MatchPhase::Pregame => enrich_pregame(client, puuid, snapshot).await,
        MatchPhase::Ingame | MatchPhase::Range => enrich_coregame(client, puuid, snapshot).await,
        _ => {}
    }
}

async fn enrich_pregame(client: &ValorantClient, puuid: &str, snapshot: &mut LiveSnapshot) {
    let Ok(player) = client.pregame_player(puuid).await else {
        return;
    };

    let Some(match_id) = str_path(&player, &["MatchID"]) else {
        return;
    };
    snapshot.match_id = Some(match_id.clone());

    let Ok(match_data) = client.pregame_match(&match_id).await else {
        return;
    };

    let player = match_data
        .pointer("/AllyTeam/Players")
        .and_then(Value::as_array)
        .and_then(|players| {
            players
                .iter()
                .find(|player| str_path(player, &["Subject"]).as_deref() == Some(puuid))
        });

    if let Some(player) = player {
        snapshot.agent_id = str_path(player, &["CharacterID"]).or(snapshot.agent_id.take());
    }
}

async fn enrich_coregame(client: &ValorantClient, puuid: &str, snapshot: &mut LiveSnapshot) {
    let Ok(player) = client.coregame_player(puuid).await else {
        return;
    };

    let Some(match_id) = str_path(&player, &["MatchID"]) else {
        return;
    };
    snapshot.match_id = Some(match_id.clone());

    let Ok(match_data) = client.coregame_match(&match_id).await else {
        return;
    };

    snapshot.map_id = str_path(&match_data, &["MapID"]).or(snapshot.map_id.take());

    let player = match_data
        .get("Players")
        .and_then(Value::as_array)
        .and_then(|players| {
            players
                .iter()
                .find(|player| str_path(player, &["Subject"]).as_deref() == Some(puuid))
        });

    if let Some(player) = player {
        snapshot.agent_id = str_path(player, &["CharacterID"]).or(snapshot.agent_id.take());
    }
}

fn first_str(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| str_path(value, path))
}

fn first_u32(value: &Value, paths: &[&[&str]]) -> Option<u32> {
    paths.iter().find_map(|path| {
        u32_path(value, path)
            .or_else(|| i32_path(value, path).and_then(|value| u32::try_from(value).ok()))
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use serde_json::json;

    use super::{
        cache_is_fresh, file_signature, match_phase, normalize_live_snapshot, session_cache_ttl,
        IDENTITY_CACHE_TTL,
    };
    use crate::riot::state::{MatchPhase, PlayerIdentity};
    use crate::riot::{
        local_client::{ExternalSession, ExternalSessions, LaunchConfiguration},
        valorant_client::ValorantContentCache,
    };

    #[test]
    fn uses_nested_session_loop_state() {
        let presence = json!({
            "matchPresenceData": { "sessionLoopState": "INGAME" },
            "provisioningFlow": "ShootingRange"
        });

        assert_eq!(match_phase(&presence), MatchPhase::Range);
    }

    #[test]
    fn falls_back_to_party_presence_state() {
        let presence = json!({
            "sessionLoopState": "MENUS",
            "partyPresenceData": { "partyState": "MATCHMAKING" }
        });

        assert_eq!(match_phase(&presence), MatchPhase::Matchmaking);
    }

    #[test]
    fn normalizes_party_and_score_fields() {
        let presence = json!({
            "sessionLoopState": "INGAME",
            "partySize": 2,
            "maxPartySize": 5,
            "partyAccessibility": "CLOSED",
            "partyOwnerMatchScoreAllyTeam": 7,
            "partyOwnerMatchScoreEnemyTeam": 4
        });

        let snapshot = normalize_live_snapshot(
            Some(&presence),
            PlayerIdentity::default(),
            Some("ap".to_string()),
            Some("ap".to_string()),
            None,
        );

        assert_eq!(snapshot.party.size, Some(2));
        assert_eq!(snapshot.score.expect("score").ally, 7);
    }

    #[test]
    fn lockfile_signature_changes_when_contents_change() {
        let path = std::env::temp_dir().join(format!(
            "radianite-lockfile-signature-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        fs::write(&path, "short").expect("fixture should be written");
        let before = file_signature(&path).expect("signature should load");

        fs::write(&path, "a longer lockfile fixture").expect("fixture should change");
        let after = file_signature(&path).expect("signature should reload");
        fs::remove_file(path).expect("fixture should be removed");

        assert_ne!(before, after);
    }

    #[test]
    fn session_cache_is_longer_while_valorant_is_present() {
        let riot_only = ExternalSessions::new();
        assert_eq!(
            session_cache_ttl(&riot_only),
            std::time::Duration::from_secs(3)
        );

        let mut valorant = ExternalSessions::new();
        valorant.insert(
            "valorant".to_string(),
            ExternalSession {
                product_id: Some("VALORANT".to_string()),
                launch_configuration: None,
            },
        );
        assert_eq!(
            session_cache_ttl(&valorant),
            std::time::Duration::from_secs(10)
        );
    }

    #[test]
    fn affinity_is_stable_for_the_active_valorant_session() {
        let mut source = super::PollingEventSource::new(ValorantContentCache::default());
        let mut sessions = ExternalSessions::new();
        sessions.insert(
            "valorant".to_string(),
            ExternalSession {
                product_id: Some("valorant".to_string()),
                launch_configuration: Some(LaunchConfiguration {
                    arguments: vec!["-ares-deployment=ap".to_string()],
                }),
            },
        );
        assert_eq!(source.affinity_for(&sessions).0.as_deref(), Some("ap"));

        sessions
            .get_mut("valorant")
            .expect("session")
            .launch_configuration = Some(LaunchConfiguration {
            arguments: vec!["-ares-deployment=eu".to_string()],
        });
        assert_eq!(source.affinity_for(&sessions).0.as_deref(), Some("ap"));

        source.cached_affinity = None;
        assert_eq!(source.affinity_for(&sessions).0.as_deref(), Some("eu"));
    }

    #[test]
    fn identity_cache_expires_at_five_minutes() {
        let fetched_at = std::time::Instant::now();
        assert!(cache_is_fresh(
            fetched_at,
            fetched_at + IDENTITY_CACHE_TTL - std::time::Duration::from_secs(1),
            IDENTITY_CACHE_TTL
        ));
        assert!(!cache_is_fresh(
            fetched_at,
            fetched_at + IDENTITY_CACHE_TTL,
            IDENTITY_CACHE_TTL
        ));
    }
}
