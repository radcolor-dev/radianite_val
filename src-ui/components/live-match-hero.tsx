import { useEffect, useState } from "react"
import {
  IconCircleDot,
  IconMap2,
  IconShield,
  IconSwords,
  IconUser,
  IconUsersGroup,
  IconWorld,
} from "@tabler/icons-react"

import { AppIcon } from "@/components/app-icon"
import { agentPortraitUrl, mapArt, rankIconUrl } from "@/lib/valorant-assets"
import { phaseLabel, playerName, queueLabel } from "@/lib/format"
import { cn } from "@/lib/utils"
import type { LiveSnapshot } from "@/lib/types"

const LIVE_PHASES = new Set(["pregame", "ingame", "range"])

export function LiveMatchHero({ snapshot }: { snapshot: LiveSnapshot | null }) {
  const isLive = snapshot ? LIVE_PHASES.has(snapshot.phase) : false

  const agentUrl = agentPortraitUrl(snapshot?.agentId)
  const [mapUrl, setMapUrl] = useState<string | null>(null)
  const [rankUrl, setRankUrl] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setMapUrl(null)
    setRankUrl(null)
    if (!snapshot) return

    mapArt(snapshot.mapId, snapshot.mapName).then((art) => {
      if (active) setMapUrl(art?.splash ?? null)
    })
    rankIconUrl(snapshot.rank?.tier, snapshot.rank?.iconUrl).then((url) => {
      if (active) setRankUrl(url)
    })

    return () => {
      active = false
    }
  }, [snapshot])

  if (!snapshot) {
    return (
      <section className="relative flex flex-1 flex-col overflow-hidden rounded-xl border bg-card">
        <StandbyArt />
        <div className="relative z-10 flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <AppIcon className="size-10 rounded-lg opacity-80" />
          <div>
            <p className="text-base font-semibold">Waiting for VALORANT…</p>
            <p className="text-sm text-muted-foreground">
              Launch the game to see your live match here.
            </p>
          </div>
        </div>
        <StatStrip snapshot={null} rankUrl={null} />
      </section>
    )
  }

  const score = snapshot.score
    ? `${snapshot.score.ally} – ${snapshot.score.enemy}`
    : null

  const standby = isLive ? null : standbyCopy(snapshot)

  return (
    <section className="relative flex flex-1 flex-col overflow-hidden rounded-xl border bg-card">
      <div className="relative min-h-[16rem] flex-1 overflow-hidden">
        {mapUrl ? (
          <img
            src={mapUrl}
            alt=""
            className="absolute inset-0 size-full object-cover"
          />
        ) : (
          <StandbyArt />
        )}
        <div className="absolute inset-0 bg-gradient-to-r from-background via-background/40 to-background/10" />
        <div className="absolute inset-0 bg-gradient-to-t from-card via-transparent to-transparent" />

        {agentUrl ? (
          <img
            src={agentUrl}
            alt={snapshot.agentName ?? ""}
            className="absolute bottom-0 left-0 h-[105%] max-w-[55%] object-contain object-left-bottom drop-shadow-2xl"
          />
        ) : null}

        {isLive ? (
          <div className="absolute left-4 top-4 flex items-center gap-2 rounded-md bg-primary px-2.5 py-1 text-xs font-semibold text-primary-foreground shadow-lg">
            <span className="size-1.5 animate-pulse rounded-full bg-current" />
            LIVE MATCH
          </div>
        ) : null}

        {standby ? (
          <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 px-6 text-center">
            <AppIcon className="size-10 rounded-lg opacity-80" />
            <div>
              <p className="flex items-center justify-center gap-2 text-base font-semibold">
                <span className="size-1.5 animate-pulse rounded-full bg-success" />
                {standby.title}
              </p>
              <p className="text-sm text-muted-foreground">{standby.subtitle}</p>
            </div>
          </div>
        ) : (
          <div className="absolute right-6 top-1/2 -translate-y-1/2 text-right">
            <AppIcon className="ml-auto size-10 rounded-lg opacity-90" />
            {snapshot.mapName ? (
              <>
                <p className="mt-2 text-xs font-medium tracking-[0.3em] text-muted-foreground">
                  MAP
                </p>
                <p className="text-4xl font-bold tracking-wide text-foreground/90 uppercase">
                  {snapshot.mapName}
                </p>
              </>
            ) : null}
          </div>
        )}
      </div>

      <StatStrip
        snapshot={snapshot}
        rankUrl={rankUrl}
        score={score}
      />
    </section>
  )
}

function standbyCopy(snapshot: LiveSnapshot): {
  title: string
  subtitle: string
} {
  switch (snapshot.phase) {
    case "matchmaking":
      return {
        title: "Finding a match…",
        subtitle: "Sit tight — your live match will appear here.",
      }
    case "menus":
      return {
        title: "Connected · In Menus",
        subtitle: "Start a match to see your map, agent, and score here.",
      }
    default:
      return {
        title: "Service is live",
        subtitle: "Start a match to see your live match here.",
      }
  }
}

function StatStrip({
  snapshot,
  rankUrl,
  score,
}: {
  snapshot: LiveSnapshot | null
  rankUrl: string | null
  score?: string | null
}) {
  const name = playerName(snapshot) ?? "—"
  const queue = queueLabel(snapshot?.queueId) ?? "—"
  const party = snapshot?.party.size
    ? `${snapshot.party.size} / ${snapshot.party.maxSize ?? snapshot.party.size}`
    : "—"
  const agent = snapshot?.agentName ?? "—"
  const map = snapshot?.mapName ?? "—"
  const rankName =
    snapshot?.rank?.tierName ??
    (snapshot?.rank?.tier ? `Tier ${snapshot.rank.tier}` : "Unranked")
  const rr =
    snapshot?.rank?.rankedRating !== null &&
    snapshot?.rank?.rankedRating !== undefined
      ? `${snapshot.rank.rankedRating} RR`
      : "—"
  const region = snapshot?.region
    ? snapshot.region.toUpperCase()
    : "—"

  return (
    <div className="relative z-10 border-t bg-card/95">
      <div className="grid grid-cols-2 gap-x-2 gap-y-2.5 px-4 py-3 sm:grid-cols-3 lg:grid-cols-6">
        <StatCell icon={<IconUser />} label="Player" value={name} accent />
        <StatCell icon={<IconCircleDot />} label="Queue" value={queue} />
        <StatCell icon={<IconUsersGroup />} label="Party" value={party} />
        <StatCell icon={<IconSwords />} label="Score" value={score ?? "—"} />
        <StatCell icon={<IconShield />} label="Agent" value={agent} />
        <StatCell icon={<IconMap2 />} label="Map" value={map} />
      </div>
      <div className="grid grid-cols-2 gap-x-2 gap-y-2.5 border-t px-4 py-3 sm:grid-cols-4">
        <StatCell
          icon={<IconCircleDot />}
          label="Phase"
          value={snapshot ? phaseLabel(snapshot.phase) : "—"}
          tone="primary"
        />
        <StatCell
          icon={
            rankUrl ? (
              <img src={rankUrl} alt="" className="size-5 object-contain" />
            ) : (
              <IconShield />
            )
          }
          label="Rank"
          value={rankName}
        />
        <StatCell icon={<IconCircleDot />} label="RR" value={rr} />
        <StatCell icon={<IconWorld />} label="Region" value={region} />
      </div>
    </div>
  )
}

function StatCell({
  icon,
  label,
  value,
  accent,
  tone,
}: {
  icon: React.ReactNode
  label: string
  value: string
  accent?: boolean
  tone?: "primary"
}) {
  return (
    <div className="flex min-w-0 items-center gap-2.5">
      <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted/60 text-muted-foreground [&_svg]:size-4">
        {icon}
      </span>
      <div className="min-w-0">
        <p className="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase">
          {label}
        </p>
        <p
          className={cn(
            "truncate text-sm font-semibold",
            accent && "text-foreground",
            tone === "primary" && "text-primary",
          )}
        >
          {value}
        </p>
      </div>
    </div>
  )
}

function StandbyArt() {
  return (
    <div className="absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,oklch(0.28_0.06_25/0.6),transparent_55%),radial-gradient(circle_at_80%_80%,oklch(0.25_0.05_264/0.7),transparent_50%)]" />
  )
}
