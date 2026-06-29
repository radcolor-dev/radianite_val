use std::env;

use chrono::DateTime;
use discord_rich_presence::{
    activity::{Activity, ActivityType, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use rust_i18n::t;

use crate::riot::{
    state::{LiveSnapshot, LocalizedMessage, MatchPhase, RpcPreview, RpcStatus},
    valorant_client::ValorantContent,
};

const DEFAULT_DISCORD_APP_ID: &str = "1520041097945153566";
const DEFAULT_GITHUB_URL: &str = "https://github.com/radcolor-dev/radianite_val";

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub application_id: Option<String>,
    pub github_url: Option<String>,
    pub assets: RpcAssetConfig,
}

#[derive(Debug, Clone)]
pub struct RpcAssetConfig {
    pub large_game: String,
    pub small_menu: String,
    pub map_prefix: String,
    pub agent_prefix: String,
    pub mode_prefix: String,
    pub small_rank_prefix: String,
}

impl RpcConfig {
    pub fn from_env() -> Self {
        Self {
            application_id: env::var("RADIANITE_DISCORD_APP_ID")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    let value = DEFAULT_DISCORD_APP_ID.trim();
                    (!value.is_empty()).then(|| value.to_string())
                }),
            github_url: env::var("RADIANITE_GITHUB_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .or_else(|| Some(DEFAULT_GITHUB_URL.to_string())),
            assets: RpcAssetConfig {
                large_game: env::var("RADIANITE_DISCORD_ASSET_GAME")
                    .unwrap_or_else(|_| "game_icon".to_string()),
                small_menu: env::var("RADIANITE_DISCORD_ASSET_MENU")
                    .unwrap_or_else(|_| "mode_unrated".to_string()),
                map_prefix: env::var("RADIANITE_DISCORD_ASSET_MAP_PREFIX")
                    .unwrap_or_else(|_| "splash_".to_string()),
                agent_prefix: env::var("RADIANITE_DISCORD_ASSET_AGENT_PREFIX")
                    .unwrap_or_else(|_| "agent_".to_string()),
                mode_prefix: env::var("RADIANITE_DISCORD_ASSET_MODE_PREFIX")
                    .unwrap_or_else(|_| "mode_".to_string()),
                small_rank_prefix: env::var("RADIANITE_DISCORD_ASSET_RANK_PREFIX")
                    .unwrap_or_else(|_| "rank_".to_string()),
            },
        }
    }

    pub fn configured(&self) -> bool {
        self.application_id.is_some()
    }
}

pub struct DiscordRpcManager {
    config: RpcConfig,
    enabled: bool,
    client: Option<DiscordIpcClient>,
    status: RpcStatus,
    locale: String,
    content: Option<ValorantContent>,
}

impl DiscordRpcManager {
    pub fn new(config: RpcConfig) -> Self {
        let configured = config.configured();
        let locale = "en-US".to_string();
        let mut status = RpcStatus::new(
            false,
            false,
            configured,
            LocalizedMessage::key(if configured {
                "status.rpc.disabled"
            } else {
                "status.rpc.notConfigured"
            }),
        );
        status.locale = locale.clone();
        Self {
            config,
            enabled: false,
            client: None,
            status,
            locale,
            content: None,
        }
    }

    pub fn status(&self) -> RpcStatus {
        self.status.clone()
    }

    pub fn set_locale(
        &mut self,
        locale: String,
        content: Option<ValorantContent>,
        snapshot: Option<&LiveSnapshot>,
    ) -> RpcStatus {
        self.locale = locale;
        self.content = content;
        self.status.locale = self.locale.clone();
        if let Some(snapshot) = snapshot {
            if self.enabled {
                return self.update_snapshot(snapshot);
            }
            self.refresh_preview(snapshot);
        }
        self.status()
    }

    pub fn set_enabled(&mut self, enabled: bool, snapshot: Option<&LiveSnapshot>) -> RpcStatus {
        self.enabled = enabled;
        if let Some(snapshot) = snapshot {
            self.refresh_preview(snapshot);
        }

        if !enabled {
            self.disconnect();
            self.status = RpcStatus::new(
                false,
                false,
                self.config.configured(),
                LocalizedMessage::key("status.rpc.disabled"),
            );
            self.status.locale = self.locale.clone();
            if let Some(snapshot) = snapshot {
                self.refresh_preview(snapshot);
            }
            return self.status();
        }

        if !self.config.configured() {
            self.enabled = false;
            self.status = RpcStatus::new(
                false,
                false,
                false,
                LocalizedMessage::key("status.rpc.notConfigured"),
            );
            self.status.locale = self.locale.clone();
            return self.status();
        }

        match self.ensure_connected() {
            Ok(()) => {
                if let Some(snapshot) = snapshot {
                    self.update_snapshot(snapshot)
                } else {
                    self.status = RpcStatus::new(
                        true,
                        true,
                        true,
                        LocalizedMessage::key("status.rpc.waiting"),
                    );
                    self.status.locale = self.locale.clone();
                    self.status()
                }
            }
            Err(message) => {
                self.client = None;
                self.status = RpcStatus::new(
                    true,
                    false,
                    true,
                    LocalizedMessage::technical("status.rpc.connectionFailed", message),
                );
                self.status.locale = self.locale.clone();
                self.status()
            }
        }
    }

    pub fn update_snapshot(&mut self, snapshot: &LiveSnapshot) -> RpcStatus {
        if !self.enabled {
            self.refresh_preview(snapshot);
            return self.status();
        }

        if let Err(message) = self.ensure_connected() {
            self.client = None;
            self.status = RpcStatus::new(
                true,
                false,
                self.config.configured(),
                LocalizedMessage::technical("status.rpc.connectionFailed", message),
            );
            self.status.locale = self.locale.clone();
            return self.status();
        }

        let (activity, preview) = render_activity(
            snapshot,
            &self.config.assets,
            self.config.github_url.as_deref(),
            &self.locale,
            self.content.as_ref(),
        );
        let result = self
            .client
            .as_mut()
            .ok_or_else(|| "Discord IPC is not connected".to_string())
            .and_then(|client| {
                client
                    .set_activity(activity)
                    .map_err(|err| format!("Discord activity update failed: {err}"))
            });

        match result {
            Ok(()) => {
                self.status = RpcStatus::new(
                    true,
                    true,
                    self.config.configured(),
                    LocalizedMessage::key("status.rpc.updated"),
                );
            }
            Err(message) => {
                self.client = None;
                self.status = RpcStatus::new(
                    true,
                    false,
                    self.config.configured(),
                    LocalizedMessage::technical("status.rpc.updateFailed", message),
                );
            }
        }

        self.status.locale = self.locale.clone();
        self.status.preview = Some(preview);
        self.status()
    }

    fn refresh_preview(&mut self, snapshot: &LiveSnapshot) {
        let (_, preview) = render_activity(
            snapshot,
            &self.config.assets,
            self.config.github_url.as_deref(),
            &self.locale,
            self.content.as_ref(),
        );
        self.status.preview = Some(preview);
        self.status.locale = self.locale.clone();
    }

    fn ensure_connected(&mut self) -> Result<(), String> {
        if self.client.is_some() {
            return Ok(());
        }

        let application_id = self
            .config
            .application_id
            .as_deref()
            .ok_or_else(|| "Discord application ID is not configured".to_string())?;
        let mut client = DiscordIpcClient::new(application_id);
        client
            .connect()
            .map_err(|err| format!("Discord IPC connection failed: {err}"))?;
        self.client = Some(client);
        Ok(())
    }

    fn disconnect(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
    }
}

fn render_activity<'a>(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    github_url: Option<&'a str>,
    locale: &str,
    content: Option<&ValorantContent>,
) -> (Activity<'a>, RpcPreview) {
    let details = truncate_discord(&details_text(snapshot, locale, content));
    let state = truncate_discord(&state_text(snapshot, locale, content));
    let large = large_asset(snapshot, assets, locale, content);
    let small = small_asset(snapshot, assets, locale, content);
    let mut rendered_assets = Assets::new()
        .large_image(large.image)
        .large_text(truncate_discord(&large.text))
        .small_image(small.image)
        .small_text(truncate_discord(&small.text));

    if let Some(github_url) = github_url {
        rendered_assets = rendered_assets.large_url(github_url).small_url(github_url);
    }

    let mut activity = Activity::new()
        .activity_type(ActivityType::Playing)
        .name(t!("rpc.name", locale = locale).to_string())
        .details(details.clone())
        .state(state.clone())
        .assets(rendered_assets);

    if let Some(started_at) = snapshot
        .session_started_at
        .as_deref()
        .and_then(unix_millis_from_rfc3339)
    {
        activity = activity.timestamps(Timestamps::new().start(started_at));
    }

    if let Some(github_url) = github_url {
        activity = activity
            .details_url(github_url)
            .state_url(github_url)
            .buttons(vec![Button::new(
                t!("rpc.button.get", locale = locale).to_string(),
                github_url,
            )]);
    }

    let preview = RpcPreview {
        name: t!("rpc.name", locale = locale).to_string(),
        details,
        state,
        started_at: snapshot
            .session_started_at
            .as_deref()
            .and_then(unix_millis_from_rfc3339),
    };
    (activity, preview)
}

fn details_text(
    snapshot: &LiveSnapshot,
    locale: &str,
    content: Option<&ValorantContent>,
) -> String {
    let mode = mode_text(snapshot, locale);
    match snapshot.phase {
        MatchPhase::Menus => t!("rpc.phase.menu", locale = locale, mode = mode).to_string(),
        MatchPhase::Matchmaking => t!("rpc.phase.queue", locale = locale, mode = mode).to_string(),
        MatchPhase::Pregame => t!("rpc.phase.pregame", locale = locale, mode = mode).to_string(),
        MatchPhase::Ingame => {
            let location = localized_map_name(snapshot, content).unwrap_or_else(|| mode.clone());
            if let Some(score) = &snapshot.score {
                t!(
                    "rpc.phase.ingameScore",
                    locale = locale,
                    location = location,
                    mode = mode,
                    ally = score.ally,
                    enemy = score.enemy
                )
                .to_string()
            } else {
                t!(
                    "rpc.phase.ingame",
                    locale = locale,
                    location = location,
                    mode = mode
                )
                .to_string()
            }
        }
        MatchPhase::Range => t!("rpc.phase.range", locale = locale).to_string(),
        MatchPhase::Unknown => mode,
    }
}

fn state_text(snapshot: &LiveSnapshot, locale: &str, content: Option<&ValorantContent>) -> String {
    let mut parts = Vec::new();

    if let Some(rank) = &snapshot.rank {
        let rank_label = rank
            .tier
            .and_then(|tier| content.and_then(|content| content.competitive_tier_name(tier)))
            .or_else(|| rank.tier_name.clone())
            .or_else(|| {
                rank.tier
                    .map(|tier| t!("rpc.rank.tier", locale = locale, tier = tier).to_string())
            });
        if let Some(label) = rank_label {
            let label = label.to_uppercase();
            if let Some(ranked_rating) = rank.ranked_rating {
                parts.push(
                    t!(
                        "rpc.state.rank",
                        locale = locale,
                        rank = label,
                        rr = ranked_rating
                    )
                    .to_string(),
                );
            } else {
                parts.push(label);
            }
        }
    }

    if let Some(size) = snapshot.party.size {
        let max = snapshot.party.max_size.unwrap_or(size);
        let key = match size {
            1 => "rpc.party.solo",
            2 => "rpc.party.duo",
            _ => "rpc.party.group",
        };
        parts.push(t!(key, locale = locale, size = size, max = max).to_string());
    }

    if parts.is_empty() {
        t!("rpc.state.live", locale = locale).to_string()
    } else {
        parts
            .into_iter()
            .reduce(|rank, party| {
                t!(
                    "rpc.state.join",
                    locale = locale,
                    rank = rank,
                    party = party
                )
                .to_string()
            })
            .unwrap_or_default()
    }
}

fn mode_text(snapshot: &LiveSnapshot, locale: &str) -> String {
    snapshot
        .queue_id
        .as_deref()
        .map(|queue| queue_label(queue, locale))
        .unwrap_or_else(|| t!("rpc.mode.valorant", locale = locale).to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedAsset {
    image: String,
    text: String,
}

fn large_asset(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    locale: &str,
    content: Option<&ValorantContent>,
) -> RenderedAsset {
    match snapshot.phase {
        MatchPhase::Pregame => snapshot
            .agent_name
            .as_deref()
            .map(|agent| RenderedAsset {
                image: prefixed_asset(&assets.agent_prefix, agent),
                text: localized_agent_name(snapshot, content).unwrap_or_else(|| agent.to_string()),
            })
            .unwrap_or_else(|| game_asset(assets, locale)),
        MatchPhase::Ingame | MatchPhase::Range => map_asset(snapshot, assets, locale, content)
            .or_else(|| {
                snapshot.agent_name.as_deref().map(|agent| RenderedAsset {
                    image: prefixed_asset(&assets.agent_prefix, agent),
                    text: localized_agent_name(snapshot, content)
                        .unwrap_or_else(|| agent.to_string()),
                })
            })
            .unwrap_or_else(|| game_asset(assets, locale)),
        _ => game_asset(assets, locale),
    }
}

fn small_asset(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    locale: &str,
    content: Option<&ValorantContent>,
) -> RenderedAsset {
    match snapshot.phase {
        MatchPhase::Pregame => mode_asset(snapshot, assets, locale)
            .or_else(|| rank_asset(snapshot, assets, locale, content))
            .unwrap_or_else(|| menu_asset(assets, locale)),
        MatchPhase::Ingame | MatchPhase::Range => snapshot
            .agent_name
            .as_deref()
            .map(|agent| RenderedAsset {
                image: prefixed_asset(&assets.agent_prefix, agent),
                text: localized_agent_name(snapshot, content).unwrap_or_else(|| agent.to_string()),
            })
            .or_else(|| rank_asset(snapshot, assets, locale, content))
            .or_else(|| mode_asset(snapshot, assets, locale))
            .unwrap_or_else(|| menu_asset(assets, locale)),
        MatchPhase::Menus | MatchPhase::Matchmaking => {
            if snapshot.queue_id.as_deref() == Some("competitive") {
                rank_asset(snapshot, assets, locale, content)
                    .or_else(|| mode_asset(snapshot, assets, locale))
                    .unwrap_or_else(|| menu_asset(assets, locale))
            } else {
                mode_asset(snapshot, assets, locale).unwrap_or_else(|| menu_asset(assets, locale))
            }
        }
        MatchPhase::Unknown => rank_asset(snapshot, assets, locale, content)
            .unwrap_or_else(|| menu_asset(assets, locale)),
    }
}

fn game_asset(assets: &RpcAssetConfig, locale: &str) -> RenderedAsset {
    RenderedAsset {
        image: assets.large_game.clone(),
        text: t!("rpc.asset.game", locale = locale).to_string(),
    }
}

fn menu_asset(assets: &RpcAssetConfig, locale: &str) -> RenderedAsset {
    RenderedAsset {
        image: assets.small_menu.clone(),
        text: t!("rpc.asset.menu", locale = locale).to_string(),
    }
}

fn map_asset(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    locale: &str,
    content: Option<&ValorantContent>,
) -> Option<RenderedAsset> {
    if snapshot.phase == MatchPhase::Range {
        return Some(RenderedAsset {
            image: square_map_asset(&assets.map_prefix, "range"),
            text: t!("rpc.asset.range", locale = locale).to_string(),
        });
    }

    let map_name = localized_map_name(snapshot, content)?;
    let asset_name = snapshot
        .map_name
        .as_deref()
        .or_else(|| snapshot.map_id.as_deref().and_then(map_name_from_path))
        .unwrap_or(&map_name);
    Some(RenderedAsset {
        image: square_map_asset(&assets.map_prefix, asset_name),
        text: map_name,
    })
}

fn square_map_asset(prefix: &str, label: &str) -> String {
    format!("{}_square", prefixed_asset(prefix, label))
}

fn rank_asset(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    locale: &str,
    content: Option<&ValorantContent>,
) -> Option<RenderedAsset> {
    snapshot
        .rank
        .as_ref()
        .and_then(|rank| rank.tier.map(|tier| (tier, rank)))
        .map(|(tier, rank)| RenderedAsset {
            image: format!("{}{tier}", assets.small_rank_prefix),
            text: content
                .and_then(|content| content.competitive_tier_name(tier))
                .or_else(|| rank.tier_name.clone())
                .unwrap_or_else(|| t!("rpc.rank.tier", locale = locale, tier = tier).to_string()),
        })
}

fn mode_asset(
    snapshot: &LiveSnapshot,
    assets: &RpcAssetConfig,
    locale: &str,
) -> Option<RenderedAsset> {
    let queue = snapshot.queue_id.as_deref()?;
    let key = mode_asset_key(queue);

    Some(RenderedAsset {
        image: format!("{}{key}", assets.mode_prefix),
        text: queue_label(queue, locale),
    })
}

fn mode_asset_key(queue: &str) -> &'static str {
    match queue.to_ascii_lowercase().as_str() {
        "competitive" | "custom" | "newmap" | "standard" | "unrated" => "unrated",
        "deathmatch" => "deathmatch",
        "ggteam" | "gungame" | "escalation" => "ggteam",
        "onefa" | "oneforall" | "replication" => "onefa",
        "snowball" | "snowballfight" => "snowball",
        "spikerush" | "quickbomb" => "spikerush",
        "swiftplay" => "swiftplay",
        "hurm" | "teamdeathmatch" => "hurm",
        "retake" | "fortcollins" => "retake",
        "knockout" | "dodgeball" => "knockout",
        "aros" | "allrandomonesite" => "aros",
        "skirmish" => "skirmish",
        "skirmishascension" => "skirmishascension",
        "basictraining" | "npev2" => "basictraining",
        "botmatch" | "exampleplayertestbot" => "botmatch",
        _ => "discovery",
    }
}

fn queue_label(queue: &str, locale: &str) -> String {
    let key = match queue.to_ascii_lowercase().as_str() {
        "competitive" => Some("rpc.mode.competitive"),
        "unrated" => Some("rpc.mode.unrated"),
        "spikerush" => Some("rpc.mode.spikerush"),
        "deathmatch" => Some("rpc.mode.deathmatch"),
        "ggteam" | "gungame" | "escalation" => Some("rpc.mode.ggteam"),
        "onefa" | "oneforall" | "replication" => Some("rpc.mode.onefa"),
        "custom" | "" => Some("rpc.mode.custom"),
        "snowball" | "snowballfight" => Some("rpc.mode.snowball"),
        "swiftplay" => Some("rpc.mode.swiftplay"),
        "hurm" | "teamdeathmatch" => Some("rpc.mode.hurm"),
        "retake" | "fortcollins" => Some("rpc.mode.retake"),
        "knockout" | "dodgeball" => Some("rpc.mode.knockout"),
        "aros" | "allrandomonesite" => Some("rpc.mode.aros"),
        "skirmish" => Some("rpc.mode.skirmish"),
        "skirmishascension" => Some("rpc.mode.skirmishascension"),
        "basictraining" | "npev2" => Some("rpc.mode.basictraining"),
        "botmatch" | "exampleplayertestbot" => Some("rpc.mode.botmatch"),
        "newmap" => Some("rpc.mode.newmap"),
        _ => None,
    };
    key.map(|key| t!(key, locale = locale).to_string())
        .unwrap_or_else(|| queue.to_string())
}

fn localized_map_name(
    snapshot: &LiveSnapshot,
    content: Option<&ValorantContent>,
) -> Option<String> {
    snapshot
        .map_id
        .as_deref()
        .and_then(|id| content.and_then(|content| content.map_name(id)))
        .or_else(|| snapshot.map_name.clone())
        .or_else(|| {
            snapshot
                .map_id
                .as_deref()
                .and_then(map_name_from_path)
                .map(str::to_string)
        })
}

fn localized_agent_name(
    snapshot: &LiveSnapshot,
    content: Option<&ValorantContent>,
) -> Option<String> {
    snapshot
        .agent_id
        .as_deref()
        .and_then(|id| content.and_then(|content| content.agent_name(id)))
        .or_else(|| snapshot.agent_name.clone())
}

fn prefixed_asset(prefix: &str, label: &str) -> String {
    format!("{prefix}{}", discord_asset_slug(label))
}

fn discord_asset_slug(label: &str) -> String {
    label
        .chars()
        .filter_map(|character| {
            let lower = character.to_ascii_lowercase();
            lower.is_ascii_alphanumeric().then_some(lower)
        })
        .collect()
}

fn map_name_from_path(map_id: &str) -> Option<&str> {
    let segment = map_id
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .filter(|segment| !segment.eq_ignore_ascii_case("pove"))?;

    Some(match segment.to_ascii_lowercase().as_str() {
        "bonsai" => "Split",
        "canyon" => "Fracture",
        "duality" => "Bind",
        "foxtrot" => "Breeze",
        "infinity" => "Abyss",
        "jam" => "Lotus",
        "juliett" => "Sunset",
        "pitt" => "Pearl",
        "plummet" => "Summit",
        "port" => "Icebox",
        "range" | "rangev2" => "The Range",
        "rook" => "Corrode",
        "triad" => "Haven",
        "hurm_alley" => "District",
        "hurm_bowl" => "Kasbah",
        "hurm_helix" => "Drift",
        "hurm_hightide" => "Glitch",
        "hurm_yard" => "Piazza",
        _ => segment,
    })
}

fn truncate_discord(value: &str) -> String {
    const LIMIT: usize = 120;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }

    value.chars().take(LIMIT).collect()
}

fn unix_millis_from_rfc3339(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::riot::state::{
        LiveSnapshot, MatchPhase, PartySnapshot, PlayerIdentity, RankSnapshot, ScoreSnapshot,
    };

    use super::{
        details_text, large_asset, render_activity, small_asset, state_text, truncate_discord,
        RpcAssetConfig,
    };

    fn snapshot(phase: MatchPhase) -> LiveSnapshot {
        LiveSnapshot {
            phase,
            player: PlayerIdentity::default(),
            region: Some("ap".to_string()),
            shard: Some("ap".to_string()),
            queue_id: Some("competitive".to_string()),
            party: PartySnapshot {
                state: None,
                size: Some(2),
                max_size: Some(5),
                accessibility: None,
            },
            map_id: None,
            map_name: None,
            agent_id: None,
            agent_name: None,
            score: None,
            rank: None,
            match_id: None,
            session_started_at: None,
            updated_at: "now".to_string(),
        }
    }

    #[test]
    fn renders_matchmaking_details() {
        assert_eq!(
            details_text(&snapshot(MatchPhase::Matchmaking), "en-US", None),
            "Competitive / Queueing"
        );
    }

    #[test]
    fn renders_rank_and_party_state() {
        let mut snapshot = snapshot(MatchPhase::Menus);
        snapshot.party.size = Some(1);
        snapshot.rank = Some(RankSnapshot {
            tier: Some(15),
            tier_name: Some("Silver 2".to_string()),
            ranked_rating: Some(47),
            last_match_delta: None,
            leaderboard_rank: None,
            season_id: None,
            icon_url: None,
        });

        assert_eq!(
            state_text(&snapshot, "en-US", None),
            "SILVER 2 (47rr) - Solo 1/5"
        );
    }

    #[test]
    fn renders_duo_and_in_party_state() {
        let mut snapshot = snapshot(MatchPhase::Menus);
        snapshot.party.max_size = Some(5);

        snapshot.party.size = Some(2);
        assert_eq!(state_text(&snapshot, "en-US", None), "Duo 2/5");

        snapshot.party.size = Some(3);
        assert_eq!(state_text(&snapshot, "en-US", None), "In Party 3/5");
    }

    #[test]
    fn renders_ingame_map_and_score_details() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_name = Some("Ascent".to_string());
        snapshot.score = Some(ScoreSnapshot { ally: 7, enemy: 4 });

        assert_eq!(
            details_text(&snapshot, "en-US", None),
            "Ascent / Competitive (7-4)"
        );
    }

    #[test]
    fn renders_map_large_and_agent_small_for_ingame() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_name = Some("Ascent".to_string());
        snapshot.agent_name = Some("KAY/O".to_string());

        let assets = asset_config();
        let large = large_asset(&snapshot, &assets, "en-US", None);
        let small = small_asset(&snapshot, &assets, "en-US", None);

        assert_eq!(large.image, "splash_ascent_square");
        assert_eq!(large.text, "Ascent");
        assert_eq!(small.image, "agent_kayo");
        assert_eq!(small.text, "KAY/O");
    }

    #[test]
    fn derives_map_asset_from_riot_map_path() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_id = Some("/Game/Maps/Ascent/Ascent".to_string());

        let large = large_asset(&snapshot, &asset_config(), "en-US", None);

        assert_eq!(large.image, "splash_ascent_square");
        assert_eq!(large.text, "Ascent");
    }

    #[test]
    fn maps_internal_map_paths_to_public_asset_names() {
        let cases = [
            ("/Game/Maps/Jam/Jam", "splash_lotus_square", "Lotus"),
            (
                "/Game/Maps/Plummet/Plummet",
                "splash_summit_square",
                "Summit",
            ),
            ("/Game/Maps/Rook/Rook", "splash_corrode_square", "Corrode"),
            (
                "/Game/Maps/HURM/HURM_Alley/HURM_Alley",
                "splash_district_square",
                "District",
            ),
        ];

        for (map_id, image, text) in cases {
            let mut snapshot = snapshot(MatchPhase::Ingame);
            snapshot.map_id = Some(map_id.to_string());

            let large = large_asset(&snapshot, &asset_config(), "en-US", None);

            assert_eq!(large.image, image);
            assert_eq!(large.text, text);
        }
    }

    #[test]
    fn maps_newer_queue_ids_to_mode_assets() {
        let cases = [
            ("retake", "mode_retake", "Retake"),
            ("fortcollins", "mode_retake", "Retake"),
            ("dodgeball", "mode_knockout", "Knockout"),
            ("hurm", "mode_hurm", "Team Deathmatch"),
        ];

        for (queue, image, text) in cases {
            let mut snapshot = snapshot(MatchPhase::Menus);
            snapshot.queue_id = Some(queue.to_string());

            let small = small_asset(&snapshot, &asset_config(), "en-US", None);

            assert_eq!(small.image, image);
            assert_eq!(small.text, text);
        }
    }

    #[test]
    fn renders_session_start_as_activity_timestamp() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.session_started_at = Some("2026-06-26T10:11:12.345Z".to_string());

        let (activity, preview) = render_activity(&snapshot, &asset_config(), None, "en-US", None);
        let rendered = serde_json::to_value(activity).expect("serialized activity");

        assert_eq!(
            rendered.pointer("/timestamps/start"),
            Some(&Value::from(1_782_468_672_345_i64))
        );
        assert_eq!(preview.started_at, Some(1_782_468_672_345_i64));
    }

    #[test]
    fn blank_catalogs_still_use_localized_static_strings() {
        let cases = [
            ("de-DE", "Gewertet / Warteschlange"),
            ("es-ES", "Competitivo / Cola"),
            ("ja-JP", "コンペティティブ / マッチを検索中"),
        ];

        for (locale, expected) in cases {
            assert_eq!(
                details_text(&snapshot(MatchPhase::Matchmaking), locale, None),
                expected
            );
        }
    }

    #[test]
    fn truncates_unicode_by_character_not_byte() {
        let rendered = truncate_discord(&"界".repeat(121));
        assert_eq!(rendered.chars().count(), 120);
        assert!(rendered.is_char_boundary(rendered.len()));
    }

    fn asset_config() -> RpcAssetConfig {
        RpcAssetConfig {
            large_game: "game_icon".to_string(),
            small_menu: "mode_unrated".to_string(),
            map_prefix: "splash_".to_string(),
            agent_prefix: "agent_".to_string(),
            mode_prefix: "mode_".to_string(),
            small_rank_prefix: "rank_".to_string(),
        }
    }
}
