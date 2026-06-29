export type CoreStatusKind =
  | "noRiotInstall"
  | "riotClientClosed"
  | "riotClientOnly"
  | "valorantLaunching"
  | "valorantReady"
  | "authExpired"
  | "disconnected"
  | "degraded"
  | "error"

export type CoreStatus = {
  kind: CoreStatusKind
  message: LocalizedMessage
  monitored: boolean
  updatedAt: string
}

export type DiagnosticSnapshot = {
  status: CoreStatus
  riotInstallsJsonExists: boolean
  riotInstallsPath?: string | null
  lockfileExists: boolean
  lockfilePath?: string | null
  lockfilePid?: number | null
  lockfileProtocol?: string | null
  lockfilePortPresent: boolean
  localApiReady: boolean
  riotClientSessionsStatus?: number | null
  sessionProductIds: string[]
  valorantSessionPresent: boolean
  region?: string | null
  shard?: string | null
  clientVersion?: string | null
  puuidPresent: boolean
  gameName?: string | null
  gameTag?: string | null
  accessTokenReady: boolean
  entitlementTokenReady: boolean
  lastError?: string | null
  updatedAt: string
}

export type MatchPhase =
  | "menus"
  | "matchmaking"
  | "pregame"
  | "ingame"
  | "replay"
  | "range"
  | "unknown"

export type LiveSnapshot = {
  phase: MatchPhase
  player: {
    puuidPresent: boolean
    gameName?: string | null
    gameTag?: string | null
  }
  region?: string | null
  shard?: string | null
  queueId?: string | null
  party: {
    state?: string | null
    size?: number | null
    maxSize?: number | null
    accessibility?: string | null
  }
  mapId?: string | null
  mapName?: string | null
  agentId?: string | null
  agentName?: string | null
  score?: {
    ally: number
    enemy: number
  } | null
  rank?: {
    tier?: number | null
    tierName?: string | null
    rankedRating?: number | null
    lastMatchDelta?: number | null
    leaderboardRank?: number | null
    seasonId?: string | null
    iconUrl?: string | null
  } | null
  matchId?: string | null
  sessionStartedAt?: string | null
  updatedAt: string
}

export type ValorantPresentation = {
  agentName?: string | null
  agentIconUrl?: string | null
  agentPortraitUrl?: string | null
  mapName?: string | null
  mapSplashUrl?: string | null
  mapListViewIconUrl?: string | null
  rankName?: string | null
  rankIconUrl?: string | null
}

export type RpcStatus = {
  enabled: boolean
  connected: boolean
  configured: boolean
  message: LocalizedMessage
  locale: string
  preview?: RpcPreview | null
  updatedAt: string
}

export type OverlayStatus = {
  enabled: boolean
  url?: string | null
  port?: number | null
  message: LocalizedMessage
  updatedAt: string
}

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "installing"
  | "installed"
  | "error"

export type UpdaterState = {
  status: UpdaterStatus
  message: LocalizedMessage
  currentVersion?: string | null
  version?: string | null
  date?: string | null
  body?: string | null
  progress?: number | null
}

export type Settings = {
  runAtBoot: boolean
  minimizeToTray: boolean
  enableRpcOnStart: boolean
  uiLocale: string
  rpcLocale: string
}

export type AppSnapshot = {
  diagnostics: DiagnosticSnapshot
  liveSnapshot: LiveSnapshot | null
  rpcStatus: RpcStatus
  overlayStatus: OverlayStatus
}

export type SettingKey = keyof Settings

export type LocalizedMessage = {
  key: string
  args?: Record<string, string | number>
  detail?: string | null
}

export type RpcPreview = {
  name: string
  details: string
  state: string
  startedAt?: number | null
}
