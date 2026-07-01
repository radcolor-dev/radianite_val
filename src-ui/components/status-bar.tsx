import {
  IconClockHour4,
  IconFileText,
  IconRefreshDot,
  IconWifi,
} from "@tabler/icons-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"
import { useTranslation } from "react-i18next"

import { Button } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import { RelativeTime } from "@/components/relative-time"
import { formatUptime } from "@/lib/format"
import { cn } from "@/lib/utils"
import type { CoreStatus } from "@/lib/types"

function connectionHealth(status: CoreStatus): {
  key: string
  tone: string
} {
  switch (status.kind) {
    case "valorantReady":
      return { key: "statusBar.excellent", tone: "text-success" }
    case "riotClientOnly":
    case "valorantLaunching":
      return { key: "statusBar.good", tone: "text-chart-4" }
    case "degraded":
      return { key: "statusBar.degraded", tone: "text-chart-4" }
    case "error":
    case "authExpired":
    case "noRiotInstall":
      return { key: "statusBar.error", tone: "text-destructive" }
    default:
      return { key: "statusBar.offline", tone: "text-muted-foreground" }
  }
}

export function StatusBar({
  status,
  lastSync,
  startedAt,
}: {
  status: CoreStatus
  lastSync: Date | null
  startedAt: number
}) {
  const { t } = useTranslation()
  const health = connectionHealth(status)
  const [uptimeMs, setUptimeMs] = useState(() => Date.now() - startedAt)

  useEffect(() => {
    const timer = window.setInterval(() => {
      setUptimeMs(Date.now() - startedAt)
    }, 1000)
    return () => window.clearInterval(timer)
  }, [startedAt])

  return (
    <footer className="flex h-11 shrink-0 items-center justify-between gap-4 border-t bg-background/80 px-4 text-xs backdrop-blur">
      <div className="flex items-center gap-4 text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <IconWifi className={cn("size-4", health.tone)} />
          {t("statusBar.connectionHealth")}
          <span className={cn("font-semibold", health.tone)}>
            {t(health.key)}
          </span>
        </span>
        <Separator orientation="vertical" className="h-4" />
        <span className="flex items-center gap-1.5">
          <IconRefreshDot className="size-4" />
          {t("statusBar.lastSync")}
          <span className="font-mono text-foreground">
            <RelativeTime date={lastSync} fallback="--:--" />
          </span>
        </span>
        <Separator orientation="vertical" className="h-4" />
        <span className="flex items-center gap-1.5">
          <IconClockHour4 className="size-4" />
          {t("statusBar.uptime")}
          <span className="font-mono text-foreground">
            {formatUptime(uptimeMs)}
          </span>
        </span>
      </div>

      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => toast.info(t("statusBar.logsSoon"))}
        >
          <IconFileText data-icon="inline-start" />
          {t("statusBar.logs")}
        </Button>
      </div>
    </footer>
  )
}
