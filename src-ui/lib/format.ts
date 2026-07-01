import type { CoreStatusKind, LiveSnapshot, MatchPhase } from "@/lib/types"
import i18n from "@/lib/i18n"

export type StatusTone = "ready" | "pending" | "error" | "idle"

export function statusPill(kind: CoreStatusKind): {
  label: string
  tone: StatusTone
} {
  switch (kind) {
    case "valorantReady":
      return { label: i18n.t("status.pill.valorantReady"), tone: "ready" }
    case "valorantLaunching":
      return { label: i18n.t("status.pill.valorantLaunching"), tone: "pending" }
    case "riotClientOnly":
      return { label: i18n.t("status.pill.riotClientOnly"), tone: "pending" }
    case "riotClientClosed":
      return { label: i18n.t("status.pill.riotClientClosed"), tone: "idle" }
    case "noRiotInstall":
      return { label: i18n.t("status.pill.noRiotInstall"), tone: "error" }
    case "authExpired":
      return { label: i18n.t("status.pill.authExpired"), tone: "error" }
    case "error":
      return { label: i18n.t("status.pill.error"), tone: "error" }
    case "degraded":
      return { label: i18n.t("status.pill.degraded"), tone: "pending" }
    case "disconnected":
    default:
      return { label: i18n.t("status.pill.disconnected"), tone: "idle" }
  }
}

export function phaseLabel(phase: MatchPhase) {
  return i18n.t(`match.phaseLabel.${phase}`, { defaultValue: labelize(phase) })
}

export function queueLabel(queueId?: string | null, queueKey?: string | null) {
  if (!queueId) return null
  const key = queueKey ?? queueId.toLowerCase()
  return i18n.t(`match.queueLabel.${key}`, { defaultValue: labelize(queueId) })
}

export function playerName(snapshot: LiveSnapshot | null) {
  const name = snapshot?.player.gameName
  if (!name) return null
  return `${name}${snapshot?.player.gameTag ? `#${snapshot.player.gameTag}` : ""}`
}

export function labelize(value: string) {
  return value
    .replace(/([A-Z])/g, " $1")
    .replace(/^\w/, (letter) => letter.toUpperCase())
    .trim()
}

export function formatTime(date: Date | null) {
  if (!date) return "--:--"
  return date.toLocaleTimeString(i18n.language, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

export function formatDate(date: Date) {
  return date.toLocaleDateString(i18n.language, {
    year: "numeric",
    month: "short",
    day: "numeric",
  })
}

export function formatUptime(ms: number) {
  const total = Math.floor(ms / 1000)
  const h = Math.floor(total / 3600)
    .toString()
    .padStart(2, "0")
  const m = Math.floor((total % 3600) / 60)
    .toString()
    .padStart(2, "0")
  const s = (total % 60).toString().padStart(2, "0")
  return `${h}:${m}:${s}`
}

export function formatUpdateDate(value?: string | null) {
  if (!value) return null
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleDateString(i18n.language, {
    year: "numeric",
    month: "short",
    day: "numeric",
  })
}
