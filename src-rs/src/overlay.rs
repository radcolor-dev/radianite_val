use rust_i18n::t;
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    app_state::AppState,
    riot::state::{
        now_timestamp, CoreStatus, CoreStatusKind, LiveSnapshot, LocalizedMessage, OverlayStatus,
        PlayerIdentity, RankSnapshot,
    },
};

const DEFAULT_OVERLAY_PORT: u16 = 48271;
const BIND_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySnapshot {
    pub status: &'static str,
    pub message: String,
    pub rank: Option<RankSnapshot>,
    pub player: PlayerIdentity,
    pub updated_at: String,
}

pub async fn run_overlay_server(state: AppState) {
    let listener = match bind_listener().await {
        Ok(listener) => listener,
        Err(message) => {
            state
                .set_overlay_status(OverlayStatus::new(
                    false,
                    None,
                    None,
                    LocalizedMessage::technical("status.overlay.failed", message),
                ))
                .await;
            return;
        }
    };

    let port = match listener.local_addr() {
        Ok(address) => address.port(),
        Err(err) => {
            state
                .set_overlay_status(OverlayStatus::new(
                    false,
                    None,
                    None,
                    LocalizedMessage::technical(
                        "status.overlay.failed",
                        format!("OBS overlay server address failed: {err}"),
                    ),
                ))
                .await;
            return;
        }
    };
    let url = format!("http://{BIND_HOST}:{port}/overlay/rank");

    state
        .set_overlay_status(OverlayStatus::new(
            true,
            Some(url),
            Some(port),
            LocalizedMessage::key("status.overlay.running"),
        ))
        .await;

    loop {
        let Ok((stream, _peer)) = listener.accept().await else {
            continue;
        };
        let state = state.clone();
        tokio::spawn(async move {
            let _ = handle_connection(stream, state).await;
        });
    }
}

async fn bind_listener() -> Result<TcpListener, String> {
    TcpListener::bind((BIND_HOST, DEFAULT_OVERLAY_PORT))
        .await
        .map_err(|err| {
            format!(
                "OBS overlay port {DEFAULT_OVERLAY_PORT} is already in use or unavailable: {err}"
            )
        })
}

async fn handle_connection(mut stream: TcpStream, state: AppState) -> std::io::Result<()> {
    let mut buffer = [0_u8; 4096];
    let read = stream.read(&mut buffer).await?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    match path.split('?').next().unwrap_or(path) {
        "/overlay/rank" => {
            write_response(&mut stream, 200, "text/html; charset=utf-8", OVERLAY_HTML).await
        }
        "/overlay/state" => {
            let snapshot =
                overlay_snapshot_from_parts(&state.status().await, state.live_snapshot().await);
            let body = serde_json::to_string(&snapshot).unwrap_or_else(|err| {
                format!(
                    r#"{{"status":"error","message":"overlay JSON serialization failed: {err}","rank":null,"player":{{"puuidPresent":false,"gameName":null,"gameTag":null}},"updatedAt":"{}"}}"#,
                    now_timestamp()
                )
            });
            write_response(&mut stream, 200, "application/json; charset=utf-8", &body).await
        }
        "/favicon.ico" => write_response(&mut stream, 204, "text/plain; charset=utf-8", "").await,
        _ => write_response(&mut stream, 404, "text/plain; charset=utf-8", "Not found").await,
    }
}

pub fn overlay_snapshot_from_parts(
    core_status: &CoreStatus,
    live_snapshot: Option<LiveSnapshot>,
) -> OverlaySnapshot {
    if let Some(snapshot) = live_snapshot {
        if let Some(rank) = snapshot.rank {
            return OverlaySnapshot {
                status: "ready",
                message: t!("overlay.rankAvailable").to_string(),
                rank: Some(rank),
                player: snapshot.player,
                updated_at: snapshot.updated_at,
            };
        }

        return OverlaySnapshot {
            status: "waiting",
            message: t!("overlay.waitingRank").to_string(),
            rank: None,
            player: snapshot.player,
            updated_at: snapshot.updated_at,
        };
    }

    let status = match core_status.kind {
        CoreStatusKind::AuthExpired | CoreStatusKind::Error => "error",
        _ => "waiting",
    };

    OverlaySnapshot {
        status,
        message: core_status
            .message
            .detail
            .clone()
            .unwrap_or_else(|| t!(core_status.message.key.as_str()).to_string()),
        rank: None,
        player: PlayerIdentity::default(),
        updated_at: core_status.updated_at.clone(),
    }
}

async fn write_response(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status_code {
        200 => "OK",
        204 => "No Content",
        404 => "Not Found",
        _ => "OK",
    };
    let bytes = body.as_bytes();
    let response = format!(
        "HTTP/1.1 {status_code} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store, no-cache, must-revalidate\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        bytes.len()
    );

    stream.write_all(response.as_bytes()).await?;
    stream.write_all(bytes).await?;
    stream.shutdown().await
}

const OVERLAY_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Radianite Rank Overlay</title>
  <style>
    :root {
      color-scheme: dark;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: transparent;
    }

    * {
      box-sizing: border-box;
    }

    html,
    body {
      width: 100%;
      height: 100%;
      margin: 0;
      overflow: hidden;
      background: transparent;
    }

    body {
      display: flex;
      align-items: center;
      justify-content: flex-start;
      padding: 8px;
    }

    .card {
      width: 344px;
      min-height: 74px;
      display: grid;
      grid-template-columns: 64px minmax(0, 1fr);
      gap: 10px;
      align-items: center;
      padding: 8px 10px 8px 8px;
      background: linear-gradient(90deg, rgba(17, 20, 28, 0.92), rgba(25, 29, 39, 0.86));
      border: 1px solid rgba(255, 255, 255, 0.12);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
      color: #f7f8fb;
    }

    .iconWrap {
      width: 60px;
      height: 60px;
      display: grid;
      place-items: center;
      background: rgba(255, 255, 255, 0.06);
      border: 1px solid rgba(255, 255, 255, 0.08);
    }

    .icon {
      width: 54px;
      height: 54px;
      object-fit: contain;
      filter: drop-shadow(0 3px 8px rgba(0, 0, 0, 0.5));
    }

    .fallbackIcon {
      width: 34px;
      height: 34px;
      border: 6px solid rgba(138, 92, 246, 0.88);
      transform: rotate(45deg);
    }

    .content {
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .topline {
      min-width: 0;
      display: flex;
      align-items: baseline;
      gap: 8px;
      font-weight: 900;
      line-height: 1;
      text-shadow: 0 2px 4px rgba(0, 0, 0, 0.55);
    }

    .rank {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 21px;
      letter-spacing: 0;
    }

    .rr {
      flex: 0 0 auto;
      font-size: 20px;
      color: #ffffff;
    }

    .subline {
      min-height: 16px;
      font-size: 12px;
      font-weight: 800;
      color: rgba(255, 255, 255, 0.82);
      text-transform: uppercase;
      letter-spacing: 0;
    }

    .delta {
      width: max-content;
      max-width: 100%;
      padding: 3px 8px;
      font-size: 13px;
      font-weight: 900;
      line-height: 1;
      color: #fff;
      background: rgba(94, 234, 165, 0.9);
    }

    .delta.negative {
      background: rgba(255, 50, 64, 0.96);
    }

    .delta.neutral {
      background: rgba(255, 255, 255, 0.14);
      color: rgba(255, 255, 255, 0.78);
    }

    .waiting .card {
      opacity: 0.88;
    }
  </style>
</head>
<body>
  <section class="card" aria-label="Radianite rank overlay">
    <div class="iconWrap">
      <img class="icon" id="rankIcon" alt="" hidden />
      <div class="fallbackIcon" id="fallbackIcon"></div>
    </div>
    <div class="content">
      <div class="topline">
        <div class="rank" id="rankName">Waiting</div>
        <div class="rr" id="rankRR"></div>
      </div>
      <div class="subline" id="subline">Radianite overlay</div>
      <div class="delta neutral" id="delta">Waiting for rank</div>
    </div>
  </section>
  <script>
    const rankIcon = document.getElementById("rankIcon");
    const fallbackIcon = document.getElementById("fallbackIcon");
    const rankName = document.getElementById("rankName");
    const rankRR = document.getElementById("rankRR");
    const subline = document.getElementById("subline");
    const delta = document.getElementById("delta");

    function setWaiting(message) {
      document.body.classList.add("waiting");
      rankIcon.hidden = true;
      fallbackIcon.hidden = false;
      rankName.textContent = "Waiting";
      rankRR.textContent = "";
      subline.textContent = "Radianite overlay";
      delta.textContent = message || "Waiting for rank";
      delta.className = "delta neutral";
    }

    function formatDelta(value) {
      if (value > 0) return "Last Match: +" + value + "pts";
      if (value < 0) return "Last Match: " + value + "pts";
      return "Last Match: 0pts";
    }

    function render(data) {
      if (!data || data.status !== "ready" || !data.rank) {
        setWaiting(data && data.message);
        return;
      }

      document.body.classList.remove("waiting");
      const rank = data.rank;
      rankName.textContent = rank.tierName || (rank.tier ? "Tier " + rank.tier : "Unrated");
      rankRR.textContent = typeof rank.rankedRating === "number" ? rank.rankedRating + "RR" : "";
      subline.textContent = data.player && data.player.gameName
        ? data.player.gameName + (data.player.gameTag ? "#" + data.player.gameTag : "")
        : "Current rank";

      if (rank.iconUrl) {
        rankIcon.src = rank.iconUrl;
        rankIcon.hidden = false;
        fallbackIcon.hidden = true;
      } else {
        rankIcon.hidden = true;
        fallbackIcon.hidden = false;
      }

      if (typeof rank.lastMatchDelta === "number") {
        delta.textContent = formatDelta(rank.lastMatchDelta);
        delta.className = rank.lastMatchDelta < 0 ? "delta negative" : "delta";
      } else {
        delta.textContent = "Last Match: unavailable";
        delta.className = "delta neutral";
      }
    }

    async function refresh() {
      try {
        const response = await fetch("/overlay/state", { cache: "no-store" });
        render(await response.json());
      } catch (_err) {
        setWaiting("Overlay disconnected");
      }
    }

    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::overlay_snapshot_from_parts;
    use crate::riot::state::{
        CoreStatus, CoreStatusKind, LiveSnapshot, MatchPhase, PartySnapshot, PlayerIdentity,
        RankSnapshot,
    };

    fn live_snapshot() -> LiveSnapshot {
        LiveSnapshot {
            phase: MatchPhase::Menus,
            player: PlayerIdentity {
                puuid_present: true,
                game_name: Some("name".to_string()),
                game_tag: Some("tag".to_string()),
            },
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
            map_id: None,
            map_name: None,
            agent_id: None,
            agent_name: None,
            score: None,
            rank: Some(RankSnapshot {
                tier: Some(24),
                tier_name: Some("Diamond 1".to_string()),
                ranked_rating: Some(20),
                last_match_delta: Some(20),
                leaderboard_rank: None,
                season_id: Some("act".to_string()),
                icon_url: Some("https://example.test/rank.png".to_string()),
            }),
            match_id: None,
            session_started_at: None,
            updated_at: "2026-06-26T10:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn renders_ready_rank_overlay_state() {
        let status = CoreStatus::new(CoreStatusKind::ValorantReady, true, "ready");
        let rendered =
            serde_json::to_value(overlay_snapshot_from_parts(&status, Some(live_snapshot())))
                .expect("overlay state should serialize");

        assert_eq!(rendered["status"], "ready");
        assert_eq!(rendered["rank"]["tierName"], "Diamond 1");
        assert_eq!(rendered["rank"]["rankedRating"], 20);
        assert_eq!(rendered["rank"]["lastMatchDelta"], 20);
        assert_eq!(rendered["rank"]["iconUrl"], "https://example.test/rank.png");
    }

    #[test]
    fn overlay_state_excludes_internal_sensitive_fields() {
        let status = CoreStatus::new(CoreStatusKind::ValorantReady, true, "ready");
        let rendered =
            serde_json::to_string(&overlay_snapshot_from_parts(&status, Some(live_snapshot())))
                .expect("overlay state should serialize");

        assert!(!rendered.contains("accessToken"));
        assert!(!rendered.contains("entitlement"));
        assert!(!rendered.contains("lockfile"));
        assert!(!rendered.contains("password"));
        assert!(!rendered.contains("privatePresence"));
    }

    #[test]
    fn renders_waiting_without_rank() {
        let status = CoreStatus::new(CoreStatusKind::ValorantReady, true, "ready");
        let mut snapshot = live_snapshot();
        snapshot.rank = None;

        let rendered = serde_json::to_value(overlay_snapshot_from_parts(&status, Some(snapshot)))
            .expect("overlay state should serialize");

        assert_eq!(rendered["status"], "waiting");
        assert_eq!(rendered["rank"], json!(null));
    }
}
