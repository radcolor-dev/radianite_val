use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use serde_json::Value;

use super::{
    local_client::{EntitlementsToken, ExternalSessions},
    state::RankSnapshot,
};

const RIOT_CLIENT_PLATFORM: &str =
    "eyJwbGF0Zm9ybVR5cGUiOiJQQyIsInBsYXRmb3JtT1MiOiJXaW5kb3dzIiwicGxhdGZvcm1PU1ZlcnNpb24iOiIxMC4wLjE5MDQ1LjEuMjU2LjY0Yml0IiwicGxhdGZvcm1DaGlwc2V0IjoiVW5rbm93biJ9";

#[derive(Debug, Clone)]
pub struct ValorantHttpError {
    pub message: String,
}

impl ValorantHttpError {
    fn transport(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn http(status: reqwest::StatusCode, body: String) -> Self {
        Self {
            message: if body.is_empty() {
                format!("Valorant service returned HTTP {}", status.as_u16())
            } else {
                format!("Valorant service returned HTTP {}: {body}", status.as_u16())
            },
        }
    }
}

impl std::fmt::Display for ValorantHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValorantHttpError {}

#[derive(Clone)]
pub struct ValorantClient {
    client: reqwest::Client,
    region: String,
    shard: String,
    tokens: EntitlementsToken,
    client_version: Option<String>,
}

impl ValorantClient {
    pub fn new(
        region: String,
        shard: String,
        tokens: EntitlementsToken,
        client_version: Option<String>,
    ) -> Result<Self, ValorantHttpError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .user_agent("Radianite/0.1")
            .build()
            .map_err(|err| {
                ValorantHttpError::transport(format!("HTTP client setup failed: {err}"))
            })?;

        Ok(Self {
            client,
            region,
            shard,
            tokens,
            client_version,
        })
    }

    fn glz_base(&self) -> String {
        format!("https://glz-{}-1.{}.a.pvp.net", self.region, self.shard)
    }

    fn pd_base(&self) -> String {
        format!("https://pd.{}.a.pvp.net", self.shard)
    }

    fn headers(&self) -> Result<HeaderMap, ValorantHttpError> {
        let mut headers = HeaderMap::new();
        let auth = HeaderValue::from_str(&format!("Bearer {}", self.tokens.access_token))
            .map_err(|err| ValorantHttpError::transport(format!("invalid auth header: {err}")))?;
        headers.insert(AUTHORIZATION, auth);

        let entitlement =
            HeaderValue::from_str(&self.tokens.entitlements_token).map_err(|err| {
                ValorantHttpError::transport(format!("invalid entitlement header: {err}"))
            })?;
        headers.insert("X-Riot-Entitlements-JWT", entitlement);
        headers.insert(
            "X-Riot-ClientPlatform",
            HeaderValue::from_static(RIOT_CLIENT_PLATFORM),
        );

        if let Some(version) = &self.client_version {
            if let Ok(value) = HeaderValue::from_str(version) {
                headers.insert("X-Riot-ClientVersion", value);
            }
        }

        Ok(headers)
    }

    async fn get_value(&self, base: String, path: &str) -> Result<Value, ValorantHttpError> {
        let response = self
            .client
            .get(format!("{base}{path}"))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|err| {
                ValorantHttpError::transport(format!("Valorant request failed: {err}"))
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| ValorantHttpError::transport(format!("Valorant body failed: {err}")))?;

        if !status.is_success() {
            return Err(ValorantHttpError::http(status, body));
        }

        serde_json::from_str::<Value>(&body).map_err(|err| {
            ValorantHttpError::transport(format!("Valorant JSON parse failed: {err}"))
        })
    }

    pub async fn pregame_player(&self, puuid: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(self.glz_base(), &format!("/pregame/v1/players/{puuid}"))
            .await
    }

    pub async fn pregame_match(&self, match_id: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(self.glz_base(), &format!("/pregame/v1/matches/{match_id}"))
            .await
    }

    pub async fn coregame_player(&self, puuid: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(self.glz_base(), &format!("/core-game/v1/players/{puuid}"))
            .await
    }

    pub async fn coregame_match(&self, match_id: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(
            self.glz_base(),
            &format!("/core-game/v1/matches/{match_id}"),
        )
        .await
    }

    pub async fn content(&self) -> Result<Value, ValorantHttpError> {
        self.get_value(self.pd_base(), "/content-service/v3/content")
            .await
    }

    pub async fn mmr(&self, puuid: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(self.pd_base(), &format!("/mmr/v1/players/{puuid}"))
            .await
    }

    pub async fn competitive_updates(&self, puuid: &str) -> Result<Value, ValorantHttpError> {
        self.get_value(
            self.pd_base(),
            &format!("/mmr/v1/players/{puuid}/competitiveupdates?startIndex=0&endIndex=1&queue=competitive"),
        )
        .await
    }
}

pub async fn fetch_public_client_version() -> Result<String, ValorantHttpError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .user_agent("Radianite/0.1")
        .build()
        .map_err(|err| ValorantHttpError::transport(format!("HTTP client setup failed: {err}")))?;

    let response = client
        .get("https://valorant-api.com/v1/version")
        .send()
        .await
        .map_err(|err| ValorantHttpError::transport(format!("version request failed: {err}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| ValorantHttpError::transport(format!("version body failed: {err}")))?;

    if !status.is_success() {
        return Err(ValorantHttpError::http(status, body));
    }

    let value = serde_json::from_str::<Value>(&body)
        .map_err(|err| ValorantHttpError::transport(format!("version JSON parse failed: {err}")))?;

    str_path(&value, &["data", "riotClientVersion"])
        .or_else(|| str_path(&value, &["data", "version"]))
        .ok_or_else(|| ValorantHttpError::transport("client version is missing"))
}

#[derive(Debug, Clone, Default)]
pub struct ValorantContent {
    agents: Vec<ContentAgent>,
    maps: Vec<ContentMap>,
    competitive_tiers: Vec<ContentCompetitiveTier>,
}

#[derive(Debug, Clone)]
struct ContentAgent {
    uuid: String,
    display_name: String,
}

#[derive(Debug, Clone)]
struct ContentMap {
    map_url: String,
    display_name: String,
}

#[derive(Debug, Clone)]
struct ContentCompetitiveTier {
    tier: u32,
    name: String,
}

impl ValorantContent {
    pub fn agent_name(&self, uuid: &str) -> Option<String> {
        self.agents
            .iter()
            .find(|agent| agent.uuid.eq_ignore_ascii_case(uuid))
            .map(|agent| agent.display_name.clone())
    }

    pub fn map_name(&self, map_url: &str) -> Option<String> {
        self.maps
            .iter()
            .find(|map| map.map_url.eq_ignore_ascii_case(map_url))
            .map(|map| map.display_name.clone())
    }

    pub fn competitive_tier_name(&self, tier: u32) -> Option<String> {
        self.competitive_tiers
            .iter()
            .find(|competitive_tier| competitive_tier.tier == tier)
            .map(|competitive_tier| competitive_tier.name.clone())
    }
}

pub async fn fetch_public_content() -> Result<ValorantContent, ValorantHttpError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent("Radianite/0.1")
        .build()
        .map_err(|err| ValorantHttpError::transport(format!("HTTP client setup failed: {err}")))?;

    let agents = fetch_valorant_api::<ValorantApiList<AgentDto>>(
        &client,
        "/agents?isPlayableCharacter=true",
    )
    .await?
    .data
    .into_iter()
    .filter_map(|agent| {
        Some(ContentAgent {
            uuid: agent.uuid?,
            display_name: agent.display_name?,
        })
    })
    .collect();

    let maps = fetch_valorant_api::<ValorantApiList<MapDto>>(&client, "/maps")
        .await?
        .data
        .into_iter()
        .filter_map(|map| {
            Some(ContentMap {
                map_url: map.map_url?,
                display_name: map.display_name?,
            })
        })
        .collect();

    let competitive_tiers =
        fetch_valorant_api::<ValorantApiList<CompetitiveTierSetDto>>(&client, "/competitivetiers")
            .await?
            .data
            .into_iter()
            .last()
            .map(|set| set.tiers)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tier| {
                Some(ContentCompetitiveTier {
                    tier: tier.tier?,
                    name: tier.tier_name?,
                })
            })
            .collect();

    Ok(ValorantContent {
        agents,
        maps,
        competitive_tiers,
    })
}

async fn fetch_valorant_api<T>(
    client: &reqwest::Client,
    endpoint: &str,
) -> Result<T, ValorantHttpError>
where
    T: for<'de> Deserialize<'de>,
{
    let response = client
        .get(format!("https://valorant-api.com/v1{endpoint}"))
        .send()
        .await
        .map_err(|err| ValorantHttpError::transport(format!("content request failed: {err}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| ValorantHttpError::transport(format!("content body failed: {err}")))?;

    if !status.is_success() {
        return Err(ValorantHttpError::http(status, body));
    }

    serde_json::from_str::<T>(&body)
        .map_err(|err| ValorantHttpError::transport(format!("content JSON parse failed: {err}")))
}

#[derive(Debug, Deserialize)]
struct ValorantApiList<T> {
    #[serde(default)]
    data: Vec<T>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentDto {
    uuid: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapDto {
    map_url: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CompetitiveTierSetDto {
    #[serde(default)]
    tiers: Vec<CompetitiveTierDto>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompetitiveTierDto {
    tier: Option<u32>,
    tier_name: Option<String>,
}

pub fn extract_region_and_shard(sessions: &ExternalSessions) -> (Option<String>, Option<String>) {
    let valorant = sessions.values().find(|session| {
        session
            .product_id
            .as_deref()
            .is_some_and(|product| product.eq_ignore_ascii_case("valorant"))
    });

    let args = valorant
        .and_then(|session| session.launch_configuration.as_ref())
        .map(|launch| launch.arguments.as_slice())
        .unwrap_or(&[]);

    let region = arg_value(args, "-ares-deployment=")
        .or_else(|| arg_value(args, "-ares-region="))
        .or_else(|| arg_value(args, "-riotclient-region="))
        .map(|value| value.to_ascii_lowercase());

    let shard = arg_value(args, "-ares-shard=")
        .map(|value| value.to_ascii_lowercase())
        .or_else(|| region.as_deref().map(shard_for_region).map(str::to_string));

    (region, shard)
}

pub fn shard_for_region(region: &str) -> &'static str {
    match region.to_ascii_lowercase().as_str() {
        "br" | "latam" => "na",
        "pbe" => "na",
        "eu" => "eu",
        "kr" => "kr",
        "ap" => "ap",
        "na" => "na",
        _ => "na",
    }
}

fn arg_value(args: &[String], prefix: &str) -> Option<String> {
    args.iter()
        .find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
        .filter(|value| !value.is_empty())
}

pub fn active_season_id(content: &Value) -> Option<String> {
    content
        .get("Seasons")
        .and_then(Value::as_array)
        .and_then(|seasons| {
            seasons.iter().find(|season| {
                bool_path(season, &["IsActive"]).unwrap_or(false)
                    && str_path(season, &["Type"]).as_deref() == Some("act")
            })
        })
        .and_then(|season| str_path(season, &["ID"]))
}

pub fn rank_from_mmr(mmr: &Value, season_id: Option<&str>) -> Option<RankSnapshot> {
    let seasons = mmr
        .pointer("/QueueSkills/competitive/SeasonalInfoBySeasonID")
        .and_then(Value::as_object)?;

    let seasonal = season_id
        .and_then(|id| seasons.get(id))
        .or_else(|| seasons.values().next_back())?;

    let tier = u32_path(seasonal, &["CompetitiveTier"]);
    let ranked_rating = i32_path(seasonal, &["RankedRating"]);
    let leaderboard_rank = u32_path(seasonal, &["LeaderboardRank"]).filter(|rank| *rank > 0);

    if tier.is_none() && ranked_rating.is_none() && leaderboard_rank.is_none() {
        return None;
    }

    Some(RankSnapshot {
        tier,
        tier_name: None,
        ranked_rating,
        leaderboard_rank,
        season_id: season_id.map(str::to_string),
    })
}

pub fn rank_from_competitive_updates(updates: &Value) -> Option<RankSnapshot> {
    let match_update = updates
        .get("Matches")
        .and_then(Value::as_array)
        .and_then(|matches| matches.first())?;

    let tier = u32_path(match_update, &["TierAfterUpdate"])
        .or_else(|| u32_path(match_update, &["CompetitiveTier"]));
    let ranked_rating = i32_path(match_update, &["RankedRatingAfterUpdate"])
        .or_else(|| i32_path(match_update, &["RankedRating"]));

    if tier.is_none() && ranked_rating.is_none() {
        return None;
    }

    Some(RankSnapshot {
        tier,
        tier_name: None,
        ranked_rating,
        leaderboard_rank: None,
        season_id: None,
    })
}

pub fn str_path(value: &Value, path: &[&str]) -> Option<String> {
    let value = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    value.as_str().map(str::to_string)
}

pub fn u32_path(value: &Value, path: &[&str]) -> Option<u32> {
    let value = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

pub fn i32_path(value: &Value, path: &[&str]) -> Option<i32> {
    let value = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    value.as_i64().and_then(|value| i32::try_from(value).ok())
}

pub fn bool_path(value: &Value, path: &[&str]) -> Option<bool> {
    let value = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    value.as_bool()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{active_season_id, extract_region_and_shard, rank_from_mmr, ValorantClient};
    use crate::riot::local_client::EntitlementsToken;
    use crate::riot::local_client::{ExternalSession, LaunchConfiguration};

    #[test]
    fn extracts_region_and_fallback_shard() {
        let mut sessions = std::collections::BTreeMap::new();
        sessions.insert(
            "abc".to_string(),
            ExternalSession {
                product_id: Some("valorant".to_string()),
                launch_configuration: Some(LaunchConfiguration {
                    arguments: vec!["-ares-deployment=latam".to_string()],
                }),
            },
        );

        let (region, shard) = extract_region_and_shard(&sessions);
        assert_eq!(region.as_deref(), Some("latam"));
        assert_eq!(shard.as_deref(), Some("na"));
    }

    #[test]
    fn finds_active_act() {
        let content = json!({
            "Seasons": [
                { "ID": "episode", "IsActive": true, "Type": "episode" },
                { "ID": "act", "IsActive": true, "Type": "act" }
            ]
        });

        assert_eq!(active_season_id(&content).as_deref(), Some("act"));
    }

    #[test]
    fn parses_rank_from_mmr() {
        let mmr = json!({
            "QueueSkills": {
                "competitive": {
                    "SeasonalInfoBySeasonID": {
                        "act": {
                            "CompetitiveTier": 24,
                            "RankedRating": 53,
                            "LeaderboardRank": 0
                        }
                    }
                }
            }
        });

        let rank = rank_from_mmr(&mmr, Some("act")).expect("rank should parse");
        assert_eq!(rank.tier, Some(24));
        assert_eq!(rank.ranked_rating, Some(53));
        assert_eq!(rank.leaderboard_rank, None);
    }

    #[test]
    fn includes_required_valorant_headers() {
        let client = ValorantClient::new(
            "ap".to_string(),
            "ap".to_string(),
            EntitlementsToken {
                access_token: "access.jwt".to_string(),
                entitlements_token: "entitlement.jwt".to_string(),
            },
            Some("release-version".to_string()),
        )
        .expect("client should build");

        let headers = client.headers().expect("headers should build");

        assert!(headers.contains_key(reqwest::header::AUTHORIZATION));
        assert!(headers.contains_key("X-Riot-Entitlements-JWT"));
        assert!(headers.contains_key("X-Riot-ClientPlatform"));
        assert!(headers.contains_key("X-Riot-ClientVersion"));
    }
}
