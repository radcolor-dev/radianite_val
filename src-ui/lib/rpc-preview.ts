import type { LiveSnapshot } from "@/lib/types"

// Mirrors src-rs/src/discord_rpc.rs so the preview matches what friends see.

export type RpcPreview = {
  name: string
  details: string
  state: string
  startedAt: number | null
}

const RPC_NAME = "VALORANT w/ Radianite"

const MAP_FROM_PATH: Record<string, string> = {
  bonsai: "Split",
  canyon: "Fracture",
  duality: "Bind",
  foxtrot: "Breeze",
  infinity: "Abyss",
  jam: "Lotus",
  juliett: "Sunset",
  pitt: "Pearl",
  plummet: "Summit",
  port: "Icebox",
  range: "The Range",
  rangev2: "The Range",
  rook: "Corrode",
  triad: "Haven",
  hurm_alley: "District",
  hurm_bowl: "Kasbah",
  hurm_helix: "Drift",
  hurm_hightide: "Glitch",
  hurm_yard: "Piazza",
}

const QUEUE_LABELS: Record<string, string> = {
  competitive: "Competitive",
  unrated: "Unrated",
  spikerush: "Spike Rush",
  deathmatch: "Deathmatch",
  ggteam: "Escalation",
  gungame: "Escalation",
  escalation: "Escalation",
  onefa: "Replication",
  oneforall: "Replication",
  replication: "Replication",
  custom: "Custom",
  "": "Custom",
  snowball: "Snowball Fight",
  snowballfight: "Snowball Fight",
  swiftplay: "Swiftplay",
  hurm: "Team Deathmatch",
  teamdeathmatch: "Team Deathmatch",
  retake: "Retake",
  fortcollins: "Retake",
  knockout: "Knockout",
  dodgeball: "Knockout",
  aros: "All Random One Site",
  allrandomonesite: "All Random One Site",
  skirmish: "Skirmish",
  skirmishascension: "Skirmish: Ascension",
  basictraining: "Basic Training",
  npev2: "Basic Training",
  botmatch: "Bot Match",
  exampleplayertestbot: "Bot Match",
  newmap: "New Map",
}

function modeText(snapshot: LiveSnapshot): string {
  const queue = snapshot.queueId
  if (!queue) return "VALORANT"
  return QUEUE_LABELS[queue.toLowerCase()] ?? queue
}

function mapNameFromPath(mapId: string): string | null {
  const segment = mapId
    .split("/")
    .filter((s) => s.length > 0 && s.toLowerCase() !== "pove")
    .pop()
  if (!segment) return null
  return MAP_FROM_PATH[segment.toLowerCase()] ?? segment
}

function resolveMapName(snapshot: LiveSnapshot): string | null {
  if (snapshot.mapName) return snapshot.mapName
  if (snapshot.mapId) return mapNameFromPath(snapshot.mapId)
  return null
}

function detailsText(snapshot: LiveSnapshot): string {
  const mode = modeText(snapshot)
  switch (snapshot.phase) {
    case "menus":
      return `${mode} / In Menu`
    case "matchmaking":
      return `${mode} / Queueing`
    case "pregame":
      return `${mode} / Agent Select`
    case "ingame": {
      const location = resolveMapName(snapshot) ?? mode
      if (snapshot.score) {
        return `${location} / ${mode} (${snapshot.score.ally}-${snapshot.score.enemy})`
      }
      return `${location} / ${mode}`
    }
    case "range":
      return "The Range / Practice"
    default:
      return mode
  }
}

function stateText(snapshot: LiveSnapshot): string {
  const parts: string[] = []

  const rank = snapshot.rank
  if (rank) {
    const label =
      rank.tierName ?? (rank.tier != null ? `T${rank.tier}` : null)
    if (label) {
      const upper = label.toUpperCase()
      parts.push(
        rank.rankedRating != null ? `${upper} (${rank.rankedRating}rr)` : upper,
      )
    }
  }

  const size = snapshot.party.size
  if (size != null) {
    const max = snapshot.party.maxSize ?? size
    const partyState = size === 1 ? "Solo" : size === 2 ? "Duo" : "In Party"
    parts.push(`${partyState} ${size}/${max}`)
  }

  return parts.length ? parts.join(" - ") : "Live"
}

export function rpcPreview(snapshot: LiveSnapshot | null): RpcPreview | null {
  if (!snapshot) return null
  return {
    name: RPC_NAME,
    details: detailsText(snapshot),
    state: stateText(snapshot),
    startedAt: snapshot.sessionStartedAt
      ? Date.parse(snapshot.sessionStartedAt) || null
      : null,
  }
}
