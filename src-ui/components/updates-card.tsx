import { lazy, Suspense, useState } from "react"
import { useTranslation } from "react-i18next"
import {
  IconCalendarEvent,
  IconClockCheck,
  IconDownload,
  IconRefresh,
  IconRocket,
  IconShieldCheck,
  IconSparkles,
  IconTag,
} from "@tabler/icons-react"

import { Button } from "@/components/ui/button"
import { Panel } from "@/components/panel"
import { RelativeTime } from "@/components/relative-time"
import { formatDate } from "@/lib/format"
import { translateMessage } from "@/lib/localized-message"
import { cn } from "@/lib/utils"
import type { UpdaterState } from "@/lib/types"

const ReleaseNotesDialog = lazy(() =>
  import("@/components/release-notes-dialog").then((module) => ({
    default: module.ReleaseNotesDialog,
  })),
)

export function UpdatesCard({ updater, version, canInstall, lastChecked, onCheck, onInstall }: {
  updater: UpdaterState
  version: string | null
  canInstall: boolean
  lastChecked: Date | null
  onCheck: () => void
  onInstall: () => void
}) {
  const { t } = useTranslation()
  const checking = updater.status === "checking" || updater.status === "installing"
  const activelyChecking = updater.status === "checking"
  const current = updater.currentVersion ?? version ?? t("common.notAvailable")
  const hasUpdate = updater.status === "available" && Boolean(updater.version)
  const releaseNotes = hasUpdate ? cleanNotes(updater.body) : null
  const [selectedRelease, setSelectedRelease] = useState<"current" | "latest" | null>(null)

  return (
    <Panel icon={<IconRocket />} title={t("updates.title")}>
      <div className="flex flex-col gap-2.5">
        <div className="flex flex-col">
          <InfoRow icon={<IconTag />} label={t("updates.currentVersion")} value={`v${current}`} mono onClick={current === t("common.notAvailable") ? undefined : () => setSelectedRelease("current")} />
          <InfoRow icon={<IconRocket />} label={t("updates.status")} value={statusLabel(updater, t)} valueClassName={statusTone(updater.status)} />
          <InfoRow icon={<IconClockCheck />} label={t("updates.lastChecked")} value={<RelativeTime date={lastChecked} fallback={t("updates.never")} />} mono />
          {hasUpdate ? <InfoRow icon={<IconSparkles />} label={t("updates.latestVersion")} value={`v${updater.version}`} valueClassName="text-primary" mono onClick={() => setSelectedRelease("latest")} /> : null}
          {hasUpdate && updater.date ? <InfoRow icon={<IconCalendarEvent />} label={t("updates.released")} value={formatDate(new Date(updater.date.replace(/ \d{2}:\d{2}:\d{2}.*$/, "")))} mono /> : null}
        </div>
        {updater.status === "installing" ? (
          <div className="flex flex-col gap-1.5">
            <div className="h-1.5 overflow-hidden rounded-full bg-muted"><div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${updater.progress ?? 35}%` }} /></div>
            <p className="text-xs text-muted-foreground">{translateMessage(t, updater.message)}</p>
          </div>
        ) : <p className={cn("text-xs", updater.status === "error" ? "text-destructive" : "text-muted-foreground")}>{translateMessage(t, updater.message)}</p>}
        {releaseNotes ? <div className="rounded-lg border bg-background/40 p-3"><p className="mb-1 flex items-center gap-1.5 text-xs font-semibold text-foreground"><IconSparkles className="size-3.5 text-primary" />{t("updates.whatsNew")}</p><p className="line-clamp-3 text-xs whitespace-pre-line text-muted-foreground">{releaseNotes}</p></div> : null}
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground"><IconShieldCheck className="size-3.5 text-success" />{t("updates.signed")}</p>
        <div className="grid grid-cols-2 gap-2">
          <Button variant="outline" size="sm" onClick={onCheck} disabled={checking} aria-busy={activelyChecking}><IconRefresh data-icon="inline-start" className={cn(activelyChecking && "animate-spin")} />{t("updates.check")}</Button>
          <Button size="sm" onClick={onInstall} disabled={!canInstall || checking}><IconDownload data-icon="inline-start" />{t("updates.install")}</Button>
        </div>
      </div>
      {selectedRelease !== null ? (
        <Suspense fallback={null}>
          <ReleaseNotesDialog
            open
            onOpenChange={(open) => { if (!open) setSelectedRelease(null) }}
            fetchFromGitHub={selectedRelease === "current"}
            release={selectedRelease === "latest" ? { version: updater.version ?? "", body: updater.body, date: updater.date } : { version: current }}
          />
        </Suspense>
      ) : null}
    </Panel>
  )
}

function InfoRow({ icon, label, value, mono, valueClassName, onClick }: { icon: React.ReactNode; label: string; value: React.ReactNode; mono?: boolean; valueClassName?: string; onClick?: () => void }) {
  const content = <><span className="flex items-center gap-2.5 text-sm text-muted-foreground [&_svg]:size-4">{icon}<span className="text-foreground">{label}</span></span><span className={cn("text-sm font-semibold", mono && "font-mono", valueClassName)}>{value}</span></>
  const className = "flex w-full items-center justify-between gap-3 border-t border-border/60 py-1.5 first:border-t-0"
  return onClick ? <button type="button" className={cn(className, "rounded-sm text-start transition-colors hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring")} onClick={onClick}>{content}</button> : <div className={className}>{content}</div>
}

function cleanNotes(body?: string | null) {
  const trimmed = body?.trim()
  return trimmed && trimmed.length > 0 ? trimmed : null
}

function statusLabel(updater: UpdaterState, t: ReturnType<typeof useTranslation>["t"]) {
  if (updater.status === "available") return t("updates.availableShort", { version: updater.version })
  return t(`updates.state.${updater.status}`)
}

function statusTone(status: UpdaterState["status"]) {
  if (status === "current" || status === "installed") return "text-success"
  if (status === "available") return "text-primary"
  if (status === "error") return "text-destructive"
  return "text-muted-foreground"
}
