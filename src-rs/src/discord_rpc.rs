use std::env;

use chrono::DateTime;
use discord_rich_presence::{
    activity::{Activity, ActivityType, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

use crate::riot::state::{LiveSnapshot, MatchPhase, RpcStatus};

const DEFAULT_DISCORD_APP_ID: &str = "1520041097945153566";
const DEFAULT_GITHUB_URL: &str = "https://github.com/radcolor-dev/radiante_val";

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
}

impl DiscordRpcManager {
    pub fn new(config: RpcConfig) -> Self {
        let configured = config.configured();
        Self {
            config,
            enabled: false,
            client: None,
            status: RpcStatus::new(
                false,
                false,
                configured,
                if configured {
                    "Discord RPC is disabled"
                } else {
                    "Set RADIANITE_DISCORD_APP_ID to enable Discord RPC"
                },
            ),
        }
    }

    pub fn status(&self) -> RpcStatus {
        self.status.clone()
    }

    pub fn set_enabled(&mut self, enabled: bool, snapshot: Option<&LiveSnapshot>) -> RpcStatus {
        self.enabled = enabled;

        if !enabled {
            self.disconnect();
            self.status = RpcStatus::new(
                false,
                false,
                self.config.configured(),
                "Discord RPC is disabled",
            );
            return self.status();
        }

        if !self.config.configured() {
            self.enabled = false;
            self.status = RpcStatus::new(
                false,
                false,
                false,
                "Set RADIANITE_DISCORD_APP_ID to enable Discord RPC",
            );
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
                        "Discord RPC is connected; waiting for live VALORANT data",
                    );
                    self.status()
                }
            }
            Err(message) => {
                self.client = None;
                self.status = RpcStatus::new(true, false, true, message);
                self.status()
            }
        }
    }

    pub fn update_snapshot(&mut self, snapshot: &LiveSnapshot) -> RpcStatus {
        if !self.enabled {
            return self.status();
        }

        if let Err(message) = self.ensure_connected() {
            self.client = None;
            self.status = RpcStatus::new(true, false, self.config.configured(), message);
            return self.status();
        }

        let activity = render_activity(
            snapshot,
            &self.config.assets,
            self.config.github_url.as_deref(),
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
                self.status =
                    RpcStatus::new(true, true, self.config.configured(), "Discord RPC updated");
            }
            Err(message) => {
                self.client = None;
                self.status = RpcStatus::new(true, false, self.config.configured(), message);
            }
        }

        self.status()
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
) -> Activity<'a> {
    let details = truncate_discord(&details_text(snapshot));
    let state = truncate_discord(&state_text(snapshot));
    let large = large_asset(snapshot, assets);
    let small = small_asset(snapshot, assets);
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
        .name("VALORANT w/ Radianite")
        .details(details)
        .state(state)
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
            .buttons(vec![Button::new("Get Radianite", github_url)]);
    }

    activity
}

fn details_text(snapshot: &LiveSnapshot) -> String {
    match snapshot.phase {
        MatchPhase::Menus => format!("{} / In Menu", mode_text(snapshot)),
        MatchPhase::Matchmaking => format!("{} / Queueing", mode_text(snapshot)),
        MatchPhase::Pregame => format!("{} / Agent Select", mode_text(snapshot)),
        MatchPhase::Ingame => {
            let location = if let Some(map_name) = snapshot
                .map_name
                .as_deref()
                .or_else(|| snapshot.map_id.as_deref().and_then(map_name_from_path))
            {
                map_name.to_string()
            } else {
                mode_text(snapshot)
            };
            if let Some(score) = &snapshot.score {
                format!(
                    "{location} / {} ({}-{})",
                    mode_text(snapshot),
                    score.ally,
                    score.enemy
                )
            } else {
                format!("{location} / {}", mode_text(snapshot))
            }
        }
        MatchPhase::Range => "The Range / Practice".to_string(),
        MatchPhase::Unknown => mode_text(snapshot),
    }
}

fn state_text(snapshot: &LiveSnapshot) -> String {
    let mut parts = Vec::new();

    if let Some(rank) = &snapshot.rank {
        let rank_label = rank
            .tier_name
            .clone()
            .or_else(|| rank.tier.map(|tier| format!("T{tier}")));
        if let Some(label) = rank_label {
            let label = label.to_ascii_uppercase();
            if let Some(ranked_rating) = rank.ranked_rating {
                parts.push(format!("{label} ({ranked_rating}rr)"));
            } else {
                parts.push(label);
            }
        }
    }

    if let Some(size) = snapshot.party.size {
        let max = snapshot.party.max_size.unwrap_or(size);
        let party_state = match size {
            1 => "Solo",
            2 => "Duo",
            _ => "In Party",
        };
        parts.push(format!("{party_state} {size}/{max}"));
    }

    if parts.is_empty() {
        "Live".to_string()
    } else {
        parts.join(" - ")
    }
}

fn mode_text(snapshot: &LiveSnapshot) -> String {
    snapshot
        .queue_id
        .as_deref()
        .map(queue_label)
        .unwrap_or_else(|| "VALORANT".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedAsset {
    image: String,
    text: String,
}

fn large_asset(snapshot: &LiveSnapshot, assets: &RpcAssetConfig) -> RenderedAsset {
    match snapshot.phase {
        MatchPhase::Pregame => snapshot
            .agent_name
            .as_deref()
            .map(|agent| RenderedAsset {
                image: prefixed_asset(&assets.agent_prefix, agent),
                text: agent.to_string(),
            })
            .unwrap_or_else(|| game_asset(assets)),
        MatchPhase::Ingame | MatchPhase::Range => map_asset(snapshot, assets)
            .or_else(|| {
                snapshot.agent_name.as_deref().map(|agent| RenderedAsset {
                    image: prefixed_asset(&assets.agent_prefix, agent),
                    text: agent.to_string(),
                })
            })
            .unwrap_or_else(|| game_asset(assets)),
        _ => game_asset(assets),
    }
}

fn small_asset(snapshot: &LiveSnapshot, assets: &RpcAssetConfig) -> RenderedAsset {
    match snapshot.phase {
        MatchPhase::Pregame => mode_asset(snapshot, assets)
            .or_else(|| rank_asset(snapshot, assets))
            .unwrap_or_else(|| menu_asset(assets)),
        MatchPhase::Ingame | MatchPhase::Range => snapshot
            .agent_name
            .as_deref()
            .map(|agent| RenderedAsset {
                image: prefixed_asset(&assets.agent_prefix, agent),
                text: agent.to_string(),
            })
            .or_else(|| rank_asset(snapshot, assets))
            .or_else(|| mode_asset(snapshot, assets))
            .unwrap_or_else(|| menu_asset(assets)),
        MatchPhase::Menus | MatchPhase::Matchmaking => {
            if snapshot.queue_id.as_deref() == Some("competitive") {
                rank_asset(snapshot, assets)
                    .or_else(|| mode_asset(snapshot, assets))
                    .unwrap_or_else(|| menu_asset(assets))
            } else {
                mode_asset(snapshot, assets).unwrap_or_else(|| menu_asset(assets))
            }
        }
        MatchPhase::Unknown => rank_asset(snapshot, assets).unwrap_or_else(|| menu_asset(assets)),
    }
}

fn game_asset(assets: &RpcAssetConfig) -> RenderedAsset {
    RenderedAsset {
        image: assets.large_game.clone(),
        text: "VALORANT w/ Radianite".to_string(),
    }
}

fn menu_asset(assets: &RpcAssetConfig) -> RenderedAsset {
    RenderedAsset {
        image: assets.small_menu.clone(),
        text: "Radianite".to_string(),
    }
}

fn map_asset(snapshot: &LiveSnapshot, assets: &RpcAssetConfig) -> Option<RenderedAsset> {
    if snapshot.phase == MatchPhase::Range {
        return Some(RenderedAsset {
            image: square_map_asset(&assets.map_prefix, "range"),
            text: "The Range".to_string(),
        });
    }

    let map_name = snapshot
        .map_name
        .as_deref()
        .or_else(|| snapshot.map_id.as_deref().and_then(map_name_from_path))?;
    Some(RenderedAsset {
        image: square_map_asset(&assets.map_prefix, map_name),
        text: map_name.to_string(),
    })
}

fn square_map_asset(prefix: &str, label: &str) -> String {
    format!("{}_square", prefixed_asset(prefix, label))
}

fn rank_asset(snapshot: &LiveSnapshot, assets: &RpcAssetConfig) -> Option<RenderedAsset> {
    snapshot
        .rank
        .as_ref()
        .and_then(|rank| rank.tier.map(|tier| (tier, rank)))
        .map(|(tier, rank)| RenderedAsset {
            image: format!("{}{tier}", assets.small_rank_prefix),
            text: rank
                .tier_name
                .clone()
                .unwrap_or_else(|| format!("Tier {tier}")),
        })
}

fn mode_asset(snapshot: &LiveSnapshot, assets: &RpcAssetConfig) -> Option<RenderedAsset> {
    let queue = snapshot.queue_id.as_deref()?;
    let key = mode_asset_key(queue);

    Some(RenderedAsset {
        image: format!("{}{key}", assets.mode_prefix),
        text: queue_label(queue),
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

fn queue_label(queue: &str) -> String {
    match queue.to_ascii_lowercase().as_str() {
        "competitive" => "Competitive",
        "unrated" => "Unrated",
        "spikerush" => "Spike Rush",
        "deathmatch" => "Deathmatch",
        "ggteam" | "gungame" | "escalation" => "Escalation",
        "onefa" | "oneforall" | "replication" => "Replication",
        "custom" | "" => "Custom",
        "snowball" | "snowballfight" => "Snowball Fight",
        "swiftplay" => "Swiftplay",
        "hurm" | "teamdeathmatch" => "Team Deathmatch",
        "retake" | "fortcollins" => "Retake",
        "knockout" | "dodgeball" => "Knockout",
        "aros" | "allrandomonesite" => "All Random One Site",
        "skirmish" => "Skirmish",
        "skirmishascension" => "Skirmish: Ascension",
        "basictraining" | "npev2" => "Basic Training",
        "botmatch" | "exampleplayertestbot" => "Bot Match",
        "newmap" => "New Map",
        _ => queue,
    }
    .to_string()
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
        details_text, large_asset, render_activity, small_asset, state_text, RpcAssetConfig,
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
            details_text(&snapshot(MatchPhase::Matchmaking)),
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

        assert_eq!(state_text(&snapshot), "SILVER 2 (47rr) - Solo 1/5");
    }

    #[test]
    fn renders_duo_and_in_party_state() {
        let mut snapshot = snapshot(MatchPhase::Menus);
        snapshot.party.max_size = Some(5);

        snapshot.party.size = Some(2);
        assert_eq!(state_text(&snapshot), "Duo 2/5");

        snapshot.party.size = Some(3);
        assert_eq!(state_text(&snapshot), "In Party 3/5");
    }

    #[test]
    fn renders_ingame_map_and_score_details() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_name = Some("Ascent".to_string());
        snapshot.score = Some(ScoreSnapshot { ally: 7, enemy: 4 });

        assert_eq!(details_text(&snapshot), "Ascent / Competitive (7-4)");
    }

    #[test]
    fn renders_map_large_and_agent_small_for_ingame() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_name = Some("Ascent".to_string());
        snapshot.agent_name = Some("KAY/O".to_string());

        let assets = asset_config();
        let large = large_asset(&snapshot, &assets);
        let small = small_asset(&snapshot, &assets);

        assert_eq!(large.image, "splash_ascent_square");
        assert_eq!(large.text, "Ascent");
        assert_eq!(small.image, "agent_kayo");
        assert_eq!(small.text, "KAY/O");
    }

    #[test]
    fn derives_map_asset_from_riot_map_path() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.map_id = Some("/Game/Maps/Ascent/Ascent".to_string());

        let large = large_asset(&snapshot, &asset_config());

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

            let large = large_asset(&snapshot, &asset_config());

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

            let small = small_asset(&snapshot, &asset_config());

            assert_eq!(small.image, image);
            assert_eq!(small.text, text);
        }
    }

    #[test]
    fn renders_session_start_as_activity_timestamp() {
        let mut snapshot = snapshot(MatchPhase::Ingame);
        snapshot.session_started_at = Some("2026-06-26T10:11:12.345Z".to_string());

        let activity = render_activity(&snapshot, &asset_config(), None);
        let rendered = serde_json::to_value(activity).expect("serialized activity");

        assert_eq!(
            rendered.pointer("/timestamps/start"),
            Some(&Value::from(1_782_468_672_345_i64))
        );
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
