use std::collections::BTreeMap;

use base64::{engine::general_purpose, Engine as _};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;

use super::lockfile::RiotLockfile;

#[derive(Debug, Clone)]
pub struct LocalClientError {
    pub status: Option<u16>,
    pub message: String,
}

impl LocalClientError {
    fn transport(message: impl Into<String>) -> Self {
        Self {
            status: None,
            message: message.into(),
        }
    }

    fn http(status: StatusCode, body: String) -> Self {
        Self {
            status: Some(status.as_u16()),
            message: if body.is_empty() {
                format!("Riot local API returned HTTP {}", status.as_u16())
            } else {
                format!("Riot local API returned HTTP {}: {body}", status.as_u16())
            },
        }
    }
}

impl std::fmt::Display for LocalClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LocalClientError {}

#[derive(Clone)]
pub struct LocalClient {
    base_url: String,
    password: String,
    lockfile_pid: u32,
    client: reqwest::Client,
}

impl LocalClient {
    pub fn from_lockfile(lockfile: &RiotLockfile) -> Result<Self, LocalClientError> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(4))
            .user_agent("Radianite/0.1")
            .build()
            .map_err(|err| {
                LocalClientError::transport(format!("HTTP client setup failed: {err}"))
            })?;

        Ok(Self {
            base_url: format!("{}://127.0.0.1:{}", lockfile.protocol, lockfile.port),
            password: lockfile.password.clone(),
            lockfile_pid: lockfile.pid,
            client,
        })
    }

    pub fn matches_lockfile(&self, lockfile: &RiotLockfile) -> bool {
        self.lockfile_pid == lockfile.pid
            && self.base_url == format!("{}://127.0.0.1:{}", lockfile.protocol, lockfile.port)
            && self.password == lockfile.password
    }

    async fn get_json<T>(&self, path: &str) -> Result<(u16, T), LocalClientError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(url)
            .basic_auth("riot", Some(&self.password))
            .send()
            .await
            .map_err(|err| {
                LocalClientError::transport(format!("Riot local API request failed: {err}"))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|err| {
            LocalClientError::transport(format!("Riot local API body failed: {err}"))
        })?;

        if !status.is_success() {
            return Err(LocalClientError::http(status, body));
        }

        let parsed = serde_json::from_str::<T>(&body).map_err(|err| {
            LocalClientError::transport(format!("Riot local API JSON parse failed: {err}"))
        })?;

        Ok((status.as_u16(), parsed))
    }

    pub async fn external_sessions(&self) -> Result<SessionFetch, LocalClientError> {
        let (status, sessions) = self
            .get_json::<ExternalSessions>("/product-session/v1/external-sessions")
            .await?;

        Ok(SessionFetch { status, sessions })
    }

    pub async fn entitlements_token(&self) -> Result<EntitlementsToken, LocalClientError> {
        let (_, token) = self
            .get_json::<EntitlementsToken>("/entitlements/v1/token")
            .await?;
        Ok(token)
    }

    pub async fn chat_session(&self) -> Result<ChatSession, LocalClientError> {
        let (_, session) = self.get_json::<ChatSession>("/chat/v1/session").await?;
        Ok(session)
    }

    pub async fn own_private_presence(
        &self,
        puuid: &str,
    ) -> Result<Option<Value>, LocalClientError> {
        let (_, presences) = self.get_json::<ChatPresences>("/chat/v4/presences").await?;
        let presence = presences
            .presences
            .into_iter()
            .find(|presence| presence.puuid.as_deref() == Some(puuid));

        let Some(private) = presence.and_then(|presence| presence.private) else {
            return Ok(None);
        };

        let decoded = general_purpose::STANDARD
            .decode(private.as_bytes())
            .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(private.as_bytes()))
            .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(private.as_bytes()))
            .map_err(|err| {
                LocalClientError::transport(format!("Riot private presence decode failed: {err}"))
            })?;

        let value = serde_json::from_slice::<Value>(&decoded).map_err(|err| {
            LocalClientError::transport(format!("Riot private presence JSON parse failed: {err}"))
        })?;

        Ok(Some(value))
    }
}

pub type ExternalSessions = BTreeMap<String, ExternalSession>;

#[derive(Debug, Clone)]
pub struct SessionFetch {
    pub status: u16,
    pub sessions: ExternalSessions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSession {
    pub product_id: Option<String>,
    pub launch_configuration: Option<LaunchConfiguration>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfiguration {
    #[serde(default)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementsToken {
    #[serde(alias = "access_token")]
    pub access_token: String,
    #[serde(alias = "token")]
    #[serde(alias = "entitlements_token")]
    pub entitlements_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub puuid: Option<String>,
    #[serde(alias = "game_name")]
    pub game_name: Option<String>,
    #[serde(alias = "game_tag")]
    pub game_tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatPresences {
    #[serde(default)]
    presences: Vec<ChatPresence>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatPresence {
    puuid: Option<String>,
    private: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{EntitlementsToken, LocalClient};
    use crate::riot::lockfile::RiotLockfile;

    #[test]
    fn parses_entitlements_token_from_local_client_shape() {
        let parsed = serde_json::from_str::<EntitlementsToken>(
            r#"{
                "accessToken": "access.jwt",
                "token": "entitlement.jwt",
                "issuer": "https://auth.riotgames.com",
                "subject": "player"
            }"#,
        )
        .expect("entitlement token should parse");

        assert_eq!(parsed.access_token, "access.jwt");
        assert_eq!(parsed.entitlements_token, "entitlement.jwt");
    }

    #[test]
    fn local_client_is_reusable_for_the_same_lockfile_session() {
        let lockfile = RiotLockfile::parse(
            PathBuf::from("lockfile"),
            "Riot Client:1234:5678:secret:https",
        )
        .expect("lockfile should parse");
        let client = LocalClient::from_lockfile(&lockfile).expect("client should build");

        assert!(client.matches_lockfile(&lockfile));

        let restarted = RiotLockfile::parse(
            PathBuf::from("lockfile"),
            "Riot Client:4321:8765:new-secret:https",
        )
        .expect("lockfile should parse");
        assert!(!client.matches_lockfile(&restarted));
    }
}
