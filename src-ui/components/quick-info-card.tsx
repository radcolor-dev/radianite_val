import {
  IconBrandDiscord,
  IconBroadcast,
  IconClockHour4,
  IconInfoCircle,
  IconTargetArrow,
} from "@tabler/icons-react"
import { useTranslation } from "react-i18next"

import { Panel } from "@/components/panel"
import { RelativeTime } from "@/components/relative-time"
import { cn } from "@/lib/utils"
import type { LiveSnapshot, OverlayStatus, RpcStatus } from "@/lib/types"

const LIVE_PHASES = new Set(["pregame", "ingame", "range"])

export function QuickInfoCard({
  overlay,
  rpc,
  snapshot,
  lastSync,
}: {
  overlay: OverlayStatus
  rpc: RpcStatus
  snapshot: LiveSnapshot | null
  lastSync: Date | null
}) {
  const { t } = useTranslation()
  const tracking = snapshot ? LIVE_PHASES.has(snapshot.phase) : false

  const rows = [
    {
      icon: <IconBroadcast />,
      label: t("quickInfo.overlay"),
      value: overlay.enabled ? t("common.ready") : t("common.off"),
      good: overlay.enabled,
    },
    {
      icon: <IconBrandDiscord />,
      label: t("quickInfo.discord"),
      value: rpc.connected ? t("common.ready") : rpc.enabled ? t("common.connecting") : t("common.off"),
      good: rpc.connected,
    },
    {
      icon: <IconTargetArrow />,
      label: t("quickInfo.tracking"),
      value: tracking ? t("common.active") : t("common.idle"),
      good: tracking,
    },
    {
      icon: <IconClockHour4 />,
      label: t("quickInfo.lastSync"),
      value: <RelativeTime date={lastSync} fallback="--:--" />,
      good: undefined,
    },
  ]

  return (
    <Panel icon={<IconInfoCircle />} title={t("quickInfo.title")}>
      <div className="flex flex-col">
        {rows.map((row) => (
          <div
            key={row.label}
            className="flex items-center justify-between gap-3 border-t border-border/60 py-2 first:border-t-0"
          >
            <span className="flex items-center gap-2.5 text-sm text-muted-foreground [&_svg]:size-4">
              {row.icon}
              <span className="text-foreground">{row.label}</span>
            </span>
            <span
              className={cn(
                "text-sm font-semibold",
                row.good === true && "text-success",
                row.good === false && "text-muted-foreground",
                row.good === undefined && "font-mono text-foreground",
              )}
            >
              {row.value}
            </span>
          </div>
        ))}
      </div>
    </Panel>
  )
}
