import type { TFunction } from "i18next"
import { useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
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
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { phaseLabel, playerName, queueLabel } from "@/lib/format"
import { cn } from "@/lib/utils"
import type { LiveSnapshot, ValorantPresentation } from "@/lib/types"

const LIVE_PHASES = new Set(["pregame", "ingame", "range"])

export function LiveMatchHero({
  snapshot,
  presentation,
}: {
  snapshot: LiveSnapshot | null
  presentation: ValorantPresentation | null
}) {
  const { t } = useTranslation()
  const isLive = snapshot ? LIVE_PHASES.has(snapshot.phase) : false
  const agentUrl = presentation?.agentPortraitUrl ?? null
  const mapUrl = presentation?.mapSplashUrl ?? null
  const rankUrl = presentation?.rankIconUrl ?? snapshot?.rank?.iconUrl ?? null

  if (!snapshot) {
    return (
      <section className="relative flex flex-1 flex-col overflow-hidden rounded-xl border bg-card">
        <StandbyArt />
        <div className="relative z-10 flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <AppIcon className="size-10 rounded-lg opacity-80" />
          <div>
            <p className="text-base font-semibold">{t("match.waitingTitle")}</p>
            <p className="text-sm text-muted-foreground">{t("match.waitingDescription")}</p>
          </div>
        </div>
        <StatStrip snapshot={null} rankUrl={null} />
      </section>
    )
  }

  const score = snapshot.score
    ? `${snapshot.score.ally} – ${snapshot.score.enemy}`
    : null
  const standby = isLive ? null : standbyCopy(snapshot, t)
  const displayMapName = presentation?.mapName ?? snapshot.mapName

  return (
    <section className="relative flex flex-1 flex-col overflow-hidden rounded-xl border bg-card">
      <div className="relative min-h-[16rem] flex-1 overflow-hidden">
        {mapUrl ? (
          <img src={mapUrl} alt="" className="absolute inset-0 size-full object-cover" />
        ) : (
          <StandbyArt />
        )}
        <div className="absolute inset-0 bg-gradient-to-r from-background via-background/40 to-background/10 rtl:bg-gradient-to-l" />
        <div className="absolute inset-0 bg-gradient-to-t from-card via-transparent to-transparent" />

        {agentUrl ? (
          <img
            src={agentUrl}
            alt={presentation?.agentName ?? snapshot.agentName ?? ""}
            className="absolute bottom-0 start-0 h-[105%] max-w-[55%] object-contain object-left-bottom drop-shadow-2xl rtl:object-right-bottom"
          />
        ) : null}

        {isLive ? (
          <div className="absolute start-4 top-4 flex items-center gap-2 rounded-md bg-primary px-2.5 py-1 text-xs font-semibold text-primary-foreground shadow-lg">
            <span className="size-1.5 animate-pulse rounded-full bg-current" />
            {t("match.live")}
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
          <div className="absolute end-6 top-1/2 -translate-y-1/2 text-end">
            <AppIcon className="ms-auto size-10 rounded-lg opacity-90" />
            {displayMapName ? (
              <>
                <p className="mt-2 text-xs font-medium tracking-[0.3em] text-muted-foreground">
                  {t("match.mapHeading")}
                </p>
                <p className="text-4xl font-bold tracking-wide text-foreground/90 uppercase">
                  {displayMapName}
                </p>
              </>
            ) : null}
          </div>
        )}
      </div>

      <StatStrip snapshot={snapshot} rankUrl={rankUrl} score={score} presentation={presentation} />
    </section>
  )
}

function standbyCopy(snapshot: LiveSnapshot, t: TFunction) {
  if (snapshot.phase === "matchmaking") {
    return { title: t("match.findingTitle"), subtitle: t("match.findingDescription") }
  }
  if (snapshot.phase === "menus") {
    return { title: t("match.menusTitle"), subtitle: t("match.menusDescription") }
  }
  return { title: t("match.serviceTitle"), subtitle: t("match.serviceDescription") }
}

function StatStrip({
  snapshot,
  rankUrl,
  score,
  presentation,
}: {
  snapshot: LiveSnapshot | null
  rankUrl: string | null
  score?: string | null
  presentation?: ValorantPresentation | null
}) {
  const { t } = useTranslation()
  const empty = t("common.notAvailable")
  const name = playerName(snapshot) ?? empty
  const queue = queueLabel(snapshot?.queueId, snapshot?.queueKey) ?? empty
  const party = snapshot?.party.size
    ? `${snapshot.party.size} / ${snapshot.party.maxSize ?? snapshot.party.size}`
    : empty
  const agent = presentation?.agentName ?? snapshot?.agentName ?? empty
  const map = presentation?.mapName ?? snapshot?.mapName ?? empty
  const rankName = presentation?.rankName ?? snapshot?.rank?.tierName ??
    (snapshot?.rank?.tier ? t("match.tier", { tier: snapshot.rank.tier }) : t("match.unranked"))
  const rr = snapshot?.rank?.rankedRating != null
    ? `${snapshot.rank.rankedRating} ${t("match.rr")}`
    : empty
  const region = snapshot?.region ? snapshot.region.toUpperCase() : empty

  return (
    <div className="relative z-10 border-t bg-card/95">
      <div className="grid grid-cols-2 gap-x-2 gap-y-2.5 px-4 py-3 sm:grid-cols-3 lg:grid-cols-6">
        <StatCell icon={<IconUser />} label={t("match.player")} value={name} accent />
        <StatCell icon={<IconCircleDot />} label={t("match.queue")} value={queue} />
        <StatCell icon={<IconUsersGroup />} label={t("match.party")} value={party} />
        <StatCell icon={<IconSwords />} label={t("match.score")} value={score ?? empty} />
        <StatCell icon={<IconShield />} label={t("match.agent")} value={agent} />
        <StatCell icon={<IconMap2 />} label={t("match.map")} value={map} />
      </div>
      <div className="grid grid-cols-2 gap-x-2 gap-y-2.5 border-t px-4 py-3 sm:grid-cols-4">
        <StatCell
          icon={<IconCircleDot />}
          label={t("match.phase")}
          value={snapshot ? phaseLabel(snapshot.phase) : empty}
          tone="primary"
        />
        <StatCell
          icon={rankUrl ? <img src={rankUrl} alt="" className="size-5 object-contain" /> : <IconShield />}
          label={t("match.rank")}
          value={rankName}
        />
        <StatCell icon={<IconCircleDot />} label={t("match.rr")} value={rr} />
        <StatCell icon={<IconWorld />} label={t("match.region")} value={region} />
      </div>
    </div>
  )
}

function StatCell({ icon, label, value, accent, tone }: {
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
        <p className="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase">{label}</p>
        <TruncatedValue value={value} accent={accent} tone={tone} />
      </div>
    </div>
  )
}

function TruncatedValue({ value, accent, tone }: {
  value: string
  accent?: boolean
  tone?: "primary"
}) {
  const valueRef = useRef<HTMLParagraphElement>(null)
  const [truncated, setTruncated] = useState(false)

  useEffect(() => {
    const element = valueRef.current
    if (!element) return

    const update = () => setTruncated(element.scrollWidth > element.clientWidth)
    update()

    const observer = new ResizeObserver(update)
    observer.observe(element)
    return () => observer.disconnect()
  }, [value])

  const text = (
    <p
      ref={valueRef}
      tabIndex={truncated ? 0 : undefined}
      className={cn(
        "truncate rounded-sm text-sm font-semibold outline-none",
        truncated && "cursor-help focus-visible:ring-2 focus-visible:ring-ring",
        accent && "text-foreground",
        tone === "primary" && "text-primary",
      )}
    >
      {value}
    </p>
  )

  if (!truncated) return text

  return (
    <Tooltip>
      <TooltipTrigger asChild>{text}</TooltipTrigger>
      <TooltipContent>{value}</TooltipContent>
    </Tooltip>
  )
}

function StandbyArt() {
  return <div className="absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,oklch(0.28_0.06_25/0.6),transparent_55%),radial-gradient(circle_at_80%_80%,oklch(0.25_0.05_264/0.7),transparent_50%)]" />
}
