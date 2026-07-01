import { getCurrentWindow } from "@tauri-apps/api/window"
import {
  IconMinus,
  IconPlayerPlay,
  IconPlayerStop,
  IconRefresh,
  IconSettings,
  IconSquare,
  IconX,
} from "@tabler/icons-react"
import { useTranslation } from "react-i18next"

import { AppIcon } from "@/components/app-icon"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { statusPill, type StatusTone } from "@/lib/format"
import type { CoreStatus } from "@/lib/types"

const toneStyles: Record<StatusTone, string> = {
  ready: "border-success/30 bg-success/10 text-success",
  pending: "border-chart-4/30 bg-chart-4/10 text-chart-4",
  error: "border-destructive/30 bg-destructive/15 text-destructive",
  idle: "border-border bg-muted/40 text-muted-foreground",
}

const appWindow = "__TAURI_INTERNALS__" in window ? getCurrentWindow() : null

export function TitleBar({
  status,
  version,
  busy,
  onRefresh,
  onStartMonitor,
  onStopMonitor,
  onOpenSettings,
}: {
  status: CoreStatus
  version: string | null
  busy: boolean
  onRefresh: () => void
  onStartMonitor: () => void
  onStopMonitor: () => void
  onOpenSettings: () => void
}) {
  const { t } = useTranslation()
  const pill = statusPill(status.kind)

  return (
    <header
      data-tauri-drag-region
      className="flex h-12 shrink-0 items-center justify-between gap-3 border-b bg-background/80 px-3 backdrop-blur"
    >
      <div data-tauri-drag-region className="flex items-center gap-3">
        <AppIcon className="size-5 rounded-sm" />
        <span className="text-sm font-semibold tracking-wide">Radianite</span>
        <span className="font-mono text-xs text-muted-foreground">
          v{version ?? "—"}
        </span>
        <span
          className={cn(
            "flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium",
            toneStyles[pill.tone],
          )}
        >
          <span className="size-1.5 rounded-full bg-current" />
          {pill.label}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon-sm"
              variant="ghost"
              onClick={onOpenSettings}
              aria-label={t("titleBar.openSettings")}
              className="size-8"
            >
              <IconSettings />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("titleBar.openSettings")}</TooltipContent>
        </Tooltip>
        <Button
          size="sm"
          variant="outline"
          onClick={onRefresh}
          disabled={busy}
          className="h-8"
        >
          <IconRefresh data-icon="inline-start" />
          {t("titleBar.refresh")}
        </Button>
        {status.monitored ? (
          <Button
            size="sm"
            onClick={onStopMonitor}
            disabled={busy}
            className="h-8 bg-primary text-primary-foreground hover:bg-primary/85"
          >
            <IconPlayerStop data-icon="inline-start" />
            {t("titleBar.stopMonitoring")}
          </Button>
        ) : (
          <Button
            size="sm"
            onClick={onStartMonitor}
            disabled={busy}
            className="h-8"
          >
            <IconPlayerPlay data-icon="inline-start" />
            {t("titleBar.startMonitoring")}
          </Button>
        )}

        <div className="ms-1 flex items-center">
          <WindowButton label={t("titleBar.minimize")} onClick={() => void appWindow?.minimize()}>
            <IconMinus className="size-4" />
          </WindowButton>
          <WindowButton
            label={t("titleBar.maximize")}
            onClick={() => void appWindow?.toggleMaximize()}
          >
            <IconSquare className="size-3.5" />
          </WindowButton>
          <WindowButton
            label={t("titleBar.close")}
            onClick={() => void appWindow?.close()}
            danger
          >
            <IconX className="size-4" />
          </WindowButton>
        </div>
      </div>
    </header>
  )
}

function WindowButton({
  children,
  label,
  onClick,
  danger,
}: {
  children: React.ReactNode
  label: string
  onClick: () => void
  danger?: boolean
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={label}
          onClick={onClick}
          className={cn(
            "flex size-8 items-center justify-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
            danger && "hover:bg-destructive hover:text-destructive-foreground",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}
