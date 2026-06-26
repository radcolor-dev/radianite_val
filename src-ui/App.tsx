import { useCallback, useEffect, useMemo, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { openUrl } from "@tauri-apps/plugin-opener"
import { relaunch } from "@tauri-apps/plugin-process"
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater"
import {
  IconBrandDiscord,
  IconBroadcast,
  IconCopy,
  IconDownload,
  IconExternalLink,
  IconPlayerPlay,
  IconPlayerStop,
  IconRefresh,
  IconShieldCheck,
} from "@tabler/icons-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { cn } from "@/lib/utils"
import "./App.css"

type CoreStatusKind =
  | "noRiotInstall"
  | "riotClientClosed"
  | "riotClientOnly"
  | "valorantLaunching"
  | "valorantReady"
  | "authExpired"
  | "disconnected"
  | "degraded"
  | "error"

type CoreStatus = {
  kind: CoreStatusKind
  message: string
  monitored: boolean
  updatedAt: string
}

type DiagnosticSnapshot = {
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

type MatchPhase =
  | "menus"
  | "matchmaking"
  | "pregame"
  | "ingame"
  | "range"
  | "unknown"

type LiveSnapshot = {
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

type RpcStatus = {
  enabled: boolean
  connected: boolean
  configured: boolean
  message: string
  updatedAt: string
}

type OverlayStatus = {
  enabled: boolean
  url?: string | null
  port?: number | null
  message: string
  updatedAt: string
}

type UpdaterStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "installing"
  | "installed"
  | "error"

type UpdaterState = {
  status: UpdaterStatus
  message: string
  currentVersion?: string | null
  version?: string | null
  date?: string | null
  body?: string | null
  progress?: number | null
}

const initialStatus: CoreStatus = {
  kind: "disconnected",
  message: "Radianite monitor has not started",
  monitored: false,
  updatedAt: "",
}

const initialDiagnostics: DiagnosticSnapshot = {
  status: initialStatus,
  riotInstallsJsonExists: false,
  lockfileExists: false,
  lockfilePortPresent: false,
  localApiReady: false,
  sessionProductIds: [],
  valorantSessionPresent: false,
  puuidPresent: false,
  accessTokenReady: false,
  entitlementTokenReady: false,
  updatedAt: "",
}

const initialRpcStatus: RpcStatus = {
  enabled: false,
  connected: false,
  configured: false,
  message: "Discord RPC status has not loaded",
  updatedAt: "",
}

const initialOverlayStatus: OverlayStatus = {
  enabled: false,
  url: null,
  port: null,
  message: "OBS overlay server status has not loaded",
  updatedAt: "",
}

const initialUpdaterState: UpdaterState = {
  status: "idle",
  message: "Updates have not been checked in this session",
  progress: null,
}

function App() {
  const [diagnostics, setDiagnostics] =
    useState<DiagnosticSnapshot>(initialDiagnostics)
  const [snapshot, setSnapshot] = useState<LiveSnapshot | null>(null)
  const [rpcStatus, setRpcStatus] = useState<RpcStatus>(initialRpcStatus)
  const [overlayStatus, setOverlayStatus] =
    useState<OverlayStatus>(initialOverlayStatus)
  const [updater, setUpdater] = useState<UpdaterState>(initialUpdaterState)
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const refresh = useCallback(async () => {
    const [
      nextDiagnostics,
      nextSnapshot,
      nextRpcStatus,
      nextOverlayStatus,
    ] = await Promise.all([
      invoke<DiagnosticSnapshot>("riot_get_diagnostics"),
      invoke<LiveSnapshot | null>("riot_get_live_snapshot"),
      invoke<RpcStatus>("discord_rpc_get_status"),
      invoke<OverlayStatus>("overlay_get_status"),
    ])

    setDiagnostics(nextDiagnostics)
    setSnapshot(nextSnapshot)
    setRpcStatus(nextRpcStatus)
    setOverlayStatus(nextOverlayStatus)
  }, [])

  const runCommand = useCallback(
    async (operation: () => Promise<void>) => {
      setBusy(true)
      setError(null)
      try {
        await operation()
        await refresh()
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setBusy(false)
      }
    },
    [refresh],
  )

  const copyOverlayUrl = useCallback(async () => {
    if (!overlayStatus.url) return

    setError(null)
    try {
      await navigator.clipboard.writeText(overlayStatus.url)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [overlayStatus.url])

  const openOverlayUrl = useCallback(async () => {
    if (!overlayStatus.url) return

    setError(null)
    try {
      await openUrl(overlayStatus.url)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [overlayStatus.url])

  const checkForUpdate = useCallback(async () => {
    setError(null)
    setUpdater({
      status: "checking",
      message: "Checking GitHub Releases for a signed update",
      progress: null,
    })

    try {
      const update = await check()
      setAvailableUpdate(update)

      if (!update) {
        setUpdater({
          status: "current",
          message: "This build is up to date",
          progress: null,
        })
        return
      }

      setUpdater({
        status: "available",
        message: `Version ${update.version} is ready to install`,
        currentVersion: update.currentVersion,
        version: update.version,
        date: update.date,
        body: update.body,
        progress: null,
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setUpdater({
        status: "error",
        message,
        progress: null,
      })
      setError(message)
    }
  }, [])

  const installAvailableUpdate = useCallback(async () => {
    if (!availableUpdate) return

    setError(null)
    setUpdater((current) => ({
      ...current,
      status: "installing",
      message: `Installing version ${availableUpdate.version}`,
      progress: 0,
    }))

    let downloadedBytes = 0
    let contentLength: number | undefined
    const onDownloadEvent = (event: DownloadEvent) => {
      if (event.event === "Started") {
        downloadedBytes = 0
        contentLength = event.data.contentLength
        setUpdater((current) => ({
          ...current,
          message: contentLength
            ? `Downloading ${formatBytes(contentLength)}`
            : "Downloading update",
          progress: 0,
        }))
        return
      }

      if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength
        setUpdater((current) => ({
          ...current,
          message: contentLength
            ? `Downloaded ${formatBytes(downloadedBytes)} of ${formatBytes(contentLength)}`
            : `Downloaded ${formatBytes(downloadedBytes)}`,
          progress: contentLength
            ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
            : null,
        }))
        return
      }

      setUpdater((current) => ({
        ...current,
        message: "Installing update",
        progress: 100,
      }))
    }

    try {
      await availableUpdate.downloadAndInstall(onDownloadEvent)
      setUpdater((current) => ({
        ...current,
        status: "installed",
        message: "Update installed. Relaunching Radianite.",
        progress: 100,
      }))
      await relaunch()
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setUpdater((current) => ({
        ...current,
        status: "error",
        message,
      }))
      setError(message)
    }
  }, [availableUpdate])

  useEffect(() => {
    const unlistenCallbacks: Array<() => void> = []

    listen<CoreStatus>("riot:status", (event) => {
      setDiagnostics((current) => ({
        ...current,
        status: event.payload,
      }))
    }).then((unlisten) => unlistenCallbacks.push(unlisten))

    listen<LiveSnapshot | null>("riot:snapshot", (event) => {
      setSnapshot(event.payload)
    }).then((unlisten) => unlistenCallbacks.push(unlisten))

    listen<RpcStatus>("discord:status", (event) => {
      setRpcStatus(event.payload)
    }).then((unlisten) => unlistenCallbacks.push(unlisten))

    runCommand(async () => {
      await refresh()
      await invoke<CoreStatus>("riot_start_monitor")
    })

    const refreshTimer = window.setInterval(() => {
      refresh().catch((err) => {
        setError(err instanceof Error ? err.message : String(err))
      })
    }, 5000)

    return () => {
      window.clearInterval(refreshTimer)
      unlistenCallbacks.forEach((unlisten) => unlisten())
    }
  }, [refresh, runCommand])

  const statusVariant = useMemo(
    () => statusBadgeVariant(diagnostics.status.kind),
    [diagnostics.status.kind],
  )

  const playerName = snapshot?.player.gameName
    ? `${snapshot.player.gameName}${snapshot.player.gameTag ? `#${snapshot.player.gameTag}` : ""}`
    : "Own player"
  const overlayUrl = overlayStatus.url ?? null

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex w-full max-w-7xl flex-col gap-5 px-5 py-5">
        <header className="flex flex-col gap-4 border-b pb-4 md:flex-row md:items-start md:justify-between">
          <div className="flex min-w-0 flex-col gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="font-heading text-xl font-medium tracking-normal">
                Radianite
              </h1>
              <Badge variant={statusVariant}>{formatLabel(diagnostics.status.kind)}</Badge>
            </div>
            <p className="max-w-3xl text-xs/relaxed text-muted-foreground">
              Windows Riot local-service diagnostics, own-player VALORANT live
              snapshot, and Discord Rich Presence.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={() => runCommand(refresh)}
              disabled={busy}
            >
              <IconRefresh data-icon="inline-start" />
              Refresh
            </Button>
            {diagnostics.status.monitored ? (
              <Button
                variant="outline"
                onClick={() =>
                  runCommand(async () => {
                    await invoke<CoreStatus>("riot_stop_monitor")
                  })
                }
                disabled={busy}
              >
                <IconPlayerStop data-icon="inline-start" />
                Stop
              </Button>
            ) : (
              <Button
                onClick={() =>
                  runCommand(async () => {
                    await invoke<CoreStatus>("riot_start_monitor")
                  })
                }
                disabled={busy}
              >
                <IconPlayerPlay data-icon="inline-start" />
                Start
              </Button>
            )}
          </div>
        </header>

        {error ? (
          <Card size="sm">
            <CardHeader>
              <CardTitle>Command Error</CardTitle>
              <CardDescription>{error}</CardDescription>
            </CardHeader>
          </Card>
        ) : null}

        <section className="grid gap-4 lg:grid-cols-[minmax(0,1.1fr)_minmax(20rem,0.9fr)]">
          <div className="flex min-w-0 flex-col gap-4">
            <Card>
              <CardHeader>
                <CardTitle>Core Status</CardTitle>
                <CardDescription>{diagnostics.status.message}</CardDescription>
                <CardAction>
                  <Badge variant={diagnostics.status.monitored ? "default" : "secondary"}>
                    {diagnostics.status.monitored ? "Monitoring" : "Stopped"}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent className="grid gap-2 md:grid-cols-3">
                <Metric label="Riot API" value={readyText(diagnostics.localApiReady)} />
                <Metric
                  label="VALORANT"
                  value={readyText(diagnostics.valorantSessionPresent)}
                />
                <Metric
                  label="Auth"
                  value={readyText(
                    diagnostics.accessTokenReady &&
                      diagnostics.entitlementTokenReady,
                  )}
                />
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Live Snapshot</CardTitle>
                <CardDescription>
                  {snapshot
                    ? `${playerName} // ${formatLabel(snapshot.phase)}`
                    : "No live VALORANT snapshot yet"}
                </CardDescription>
                <CardAction>
                  <Badge variant={snapshot ? "default" : "secondary"}>
                    {snapshot ? "Live" : "Waiting"}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent className="grid gap-3 md:grid-cols-2">
                <InfoRow label="Phase" value={snapshot && formatLabel(snapshot.phase)} />
                <InfoRow label="Queue" value={snapshot?.queueId} />
                <InfoRow
                  label="Party"
                  value={
                    snapshot?.party.size
                      ? `${snapshot.party.size}/${snapshot.party.maxSize ?? snapshot.party.size}`
                      : snapshot?.party.state
                  }
                />
                <InfoRow
                  label="Score"
                  value={
                    snapshot?.score
                      ? `${snapshot.score.ally} - ${snapshot.score.enemy}`
                      : null
                  }
                />
                <InfoRow label="Map" value={snapshot?.mapName ?? snapshot?.mapId} />
                <InfoRow
                  label="Agent"
                  value={snapshot?.agentName ?? snapshot?.agentId}
                />
                <InfoRow
                  label="Rank"
                  value={
                    snapshot?.rank
                      ? `${snapshot.rank.tierName ?? `Tier ${snapshot.rank.tier ?? "?"}`}${
                          snapshot.rank.rankedRating !== null &&
                          snapshot.rank.rankedRating !== undefined
                            ? ` // ${snapshot.rank.rankedRating} RR`
                            : ""
                        }`
                      : null
                  }
                />
                <InfoRow
                  label="Region"
                  value={
                    snapshot?.region
                      ? `${snapshot.region.toUpperCase()} / ${snapshot.shard?.toUpperCase() ?? "?"}`
                      : null
                  }
                />
              </CardContent>
            </Card>
          </div>

          <div className="flex min-w-0 flex-col gap-4">
            <Card>
              <CardHeader>
                <CardTitle>OBS Overlay</CardTitle>
                <CardDescription>{overlayStatus.message}</CardDescription>
                <CardAction>
                  <Badge variant={overlayStatus.enabled ? "default" : "secondary"}>
                    {overlayStatus.enabled ? "Running" : "Stopped"}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <div className="grid min-w-0 gap-2">
                  <InfoRow label="Browser source" value={overlayUrl} />
                  <InfoRow
                    label="Suggested size"
                    value={overlayUrl ? "360 x 90" : null}
                  />
                  <InfoRow
                    label="Bind"
                    value={
                      overlayStatus.port
                        ? `127.0.0.1:${overlayStatus.port}`
                        : null
                    }
                  />
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="outline"
                    onClick={copyOverlayUrl}
                    disabled={!overlayUrl}
                  >
                    <IconCopy data-icon="inline-start" />
                    Copy URL
                  </Button>
                  <Button
                    variant="outline"
                    onClick={openOverlayUrl}
                    disabled={!overlayUrl}
                  >
                    <IconExternalLink data-icon="inline-start" />
                    Open
                  </Button>
                </div>
                <div className="border bg-[rgb(24,24,27)] p-2">
                  {overlayUrl ? (
                    <iframe
                      title="OBS rank overlay preview"
                      src={overlayUrl}
                      className="h-[90px] w-full border-0 bg-transparent"
                    />
                  ) : (
                    <div className="flex h-[90px] items-center justify-center gap-2 text-xs text-muted-foreground">
                      <IconBroadcast />
                      Overlay preview is unavailable
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Discord Rich Presence</CardTitle>
                <CardDescription>{rpcStatus.message}</CardDescription>
                <CardAction>
                  <Badge variant={rpcStatus.connected ? "default" : "secondary"}>
                    {rpcStatus.connected ? "Connected" : "Disconnected"}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <Button
                  variant={rpcStatus.enabled ? "outline" : "default"}
                  disabled={busy || (!rpcStatus.configured && !rpcStatus.enabled)}
                  onClick={() =>
                    runCommand(async () => {
                      await invoke<RpcStatus>("discord_rpc_set_enabled", {
                        enabled: !rpcStatus.enabled,
                      })
                    })
                  }
                >
                  <IconBrandDiscord data-icon="inline-start" />
                  {rpcStatus.enabled ? "Disable RPC" : "Enable RPC"}
                </Button>
                <p className="text-xs/relaxed text-muted-foreground">
                  {rpcStatus.configured
                    ? "Activity is rendered from sanitized live snapshots."
                    : "Configure RADIANITE_DISCORD_APP_ID before enabling RPC."}
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>App Updates</CardTitle>
                <CardDescription>{updater.message}</CardDescription>
                <CardAction>
                  <Badge variant={updaterBadgeVariant(updater.status)}>
                    {formatLabel(updater.status)}
                  </Badge>
                </CardAction>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <div className="grid min-w-0 gap-2">
                  <InfoRow
                    label="Installed"
                    value={updater.currentVersion}
                  />
                  <InfoRow label="Available" value={updater.version} />
                  <InfoRow label="Published" value={formatUpdateDate(updater.date)} />
                  <InfoRow label="Release notes" value={updater.body} />
                </div>
                {updater.status === "installing" ? (
                  <div className="h-2 border bg-muted">
                    <div
                      className="h-full bg-primary transition-[width]"
                      style={{
                        width:
                          updater.progress === null || updater.progress === undefined
                            ? "35%"
                            : `${updater.progress}%`,
                      }}
                    />
                  </div>
                ) : null}
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="outline"
                    onClick={checkForUpdate}
                    disabled={
                      updater.status === "checking" || updater.status === "installing"
                    }
                  >
                    <IconRefresh data-icon="inline-start" />
                    Check
                  </Button>
                  <Button
                    onClick={installAvailableUpdate}
                    disabled={
                      !availableUpdate ||
                      updater.status === "checking" ||
                      updater.status === "installing"
                    }
                  >
                    <IconDownload data-icon="inline-start" />
                    Install
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Redacted Diagnostics</CardTitle>
                <CardDescription>
                  Tokens, entitlement JWTs, and lockfile password are never shown.
                </CardDescription>
                <CardAction>
                  <IconShieldCheck className="text-muted-foreground" />
                </CardAction>
              </CardHeader>
              <CardContent className="flex flex-col gap-1">
                <InfoRow
                  label="Riot installs"
                  value={readyText(diagnostics.riotInstallsJsonExists)}
                />
                <InfoRow
                  label="Lockfile"
                  value={readyText(diagnostics.lockfileExists)}
                />
                <InfoRow
                  label="Lockfile PID"
                  value={diagnostics.lockfilePid?.toString()}
                />
                <InfoRow
                  label="Protocol"
                  value={diagnostics.lockfileProtocol}
                />
                <InfoRow
                  label="Session HTTP"
                  value={diagnostics.riotClientSessionsStatus?.toString()}
                />
                <InfoRow
                  label="Products"
                  value={
                    diagnostics.sessionProductIds.length
                      ? diagnostics.sessionProductIds.join(", ")
                      : null
                  }
                />
                <InfoRow label="Client version" value={diagnostics.clientVersion} />
                <InfoRow
                  label="Player identity"
                  value={
                    diagnostics.gameName
                      ? `${diagnostics.gameName}${diagnostics.gameTag ? `#${diagnostics.gameTag}` : ""}`
                      : readyText(diagnostics.puuidPresent)
                  }
                />
                <InfoRow label="Last error" value={diagnostics.lastError} />
              </CardContent>
            </Card>
          </div>
        </section>
      </div>
    </main>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1 border p-3">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="truncate text-sm font-medium">{value}</span>
    </div>
  )
}

function InfoRow({
  label,
  value,
}: {
  label: string
  value?: string | number | null
}) {
  return (
    <div className="grid min-w-0 grid-cols-[minmax(8rem,0.4fr)_minmax(0,1fr)] gap-3 border-t py-2 text-xs/relaxed first:border-t-0 max-sm:grid-cols-1 max-sm:gap-1">
      <span className="text-muted-foreground">{label}</span>
      <span
        className={cn(
          "min-w-0 break-words",
          value ? "text-foreground" : "text-muted-foreground",
        )}
      >
        {value || "Not available"}
      </span>
    </div>
  )
}

function statusBadgeVariant(kind: CoreStatusKind) {
  if (kind === "valorantReady") return "default"
  if (kind === "degraded" || kind === "valorantLaunching") return "outline"
  if (kind === "error" || kind === "authExpired") return "destructive"
  return "secondary"
}

function readyText(value: boolean) {
  return value ? "Ready" : "Not ready"
}

function updaterBadgeVariant(status: UpdaterStatus) {
  if (status === "available" || status === "installed") return "default"
  if (status === "checking" || status === "installing") return "outline"
  if (status === "error") return "destructive"
  return "secondary"
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`
  const kb = value / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB`
  const mb = kb / 1024
  return `${mb.toFixed(1)} MB`
}

function formatUpdateDate(value?: string | null) {
  if (!value) return null

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value

  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  })
}

function formatLabel(value: string) {
  return value
    .replace(/([A-Z])/g, " $1")
    .replace(/^\w/, (letter) => letter.toUpperCase())
}

export default App
