import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { IconBrandDiscord, IconPlayerPlay, IconPlayerStop } from "@tabler/icons-react"

import { AppIcon } from "@/components/app-icon"
import { Button } from "@/components/ui/button"
import { Panel } from "@/components/panel"
import { cn } from "@/lib/utils"
import type { LiveSnapshot, RpcPreview, RpcStatus, ValorantPresentation } from "@/lib/types"

export function DiscordCard({ rpc, snapshot, presentation, busy, onToggle }: {
  rpc: RpcStatus
  snapshot: LiveSnapshot | null
  presentation: ValorantPresentation | null
  busy: boolean
  onToggle: () => void
}) {
  const { t } = useTranslation()
  const canToggle = rpc.configured || rpc.enabled
  const preview = rpc.preview ?? null

  return (
    <Panel
      icon={<IconBrandDiscord />}
      title={t("discord.title")}
      action={
        <Button variant="outline" size="sm" onClick={onToggle} disabled={busy || !canToggle}>
          {rpc.enabled ? <IconPlayerStop data-icon="inline-start" /> : <IconPlayerPlay data-icon="inline-start" />}
          {rpc.enabled ? t("common.disable") : t("common.enable")}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2 text-sm">
          <span className={cn("size-2 rounded-full", rpc.connected ? "bg-success" : "bg-muted-foreground")} />
          <span className={cn("font-medium", rpc.connected ? "text-success" : "text-muted-foreground")}>
            {rpc.connected ? t("common.connected") : t("common.disconnected")}
          </span>
        </div>

        <p className="text-xs text-muted-foreground">{t("discord.friendsSee")}</p>
        {preview ? (
          <DiscordActivity preview={preview} snapshot={snapshot} presentation={presentation} />
        ) : (
          <div className="rounded-lg border bg-[#1a1b1e] p-3 text-sm text-muted-foreground">
            {canToggle ? t("discord.noMatch") : t("discord.notConfigured")}
          </div>
        )}
      </div>
    </Panel>
  )
}

function DiscordActivity({ preview, snapshot, presentation }: { preview: RpcPreview; snapshot: LiveSnapshot | null; presentation: ValorantPresentation | null }) {
  const { t } = useTranslation()
  const elapsed = useElapsed(preview.startedAt ?? null)
  const agentUrl = presentation?.agentIconUrl ?? null
  const largeUrl = snapshot?.phase === "pregame"
    ? agentUrl
    : snapshot?.phase === "ingame" || snapshot?.phase === "range"
      ? presentation?.mapListViewIconUrl ?? agentUrl
      : null

  return (
    <div className="rounded-lg border border-white/5 bg-[#232428] p-3 text-[#dbdee1]" dir="auto">
      <p className="mb-2 text-[0.65rem] font-bold tracking-wide text-[#b5bac1] uppercase">{t("discord.playing")}</p>
      <div className="flex gap-3">
        <div className="relative size-[52px] shrink-0">
          <div className="flex size-full items-center justify-center overflow-hidden rounded-lg bg-[#1a1b1e]">
            {largeUrl ? <img src={largeUrl} alt="" className="size-full object-cover" /> : <AppIcon className="size-9 rounded-md" />}
          </div>
          <span className="absolute -bottom-1 -end-1 flex size-5 items-center justify-center overflow-hidden rounded-full bg-[#232428] ring-2 ring-[#232428]">
            {agentUrl ? <img src={agentUrl} alt="" className="size-4 rounded-full object-cover" /> : null}
          </span>
        </div>
        <div className="min-w-0 flex-1 text-sm leading-tight">
          <p className="truncate font-semibold text-white">{preview.name}</p>
          <p className="truncate text-[0.7rem] text-[#b5bac1]">{preview.details}</p>
          <p className="truncate text-[0.7rem] text-[#b5bac1]">{preview.state}</p>
          {elapsed ? <p className="mt-0.5 truncate text-[0.8rem] text-[#b5bac1]">{t("discord.elapsed", { time: elapsed })}</p> : null}
        </div>
      </div>
    </div>
  )
}

function useElapsed(startedAt: number | null) {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    if (startedAt == null) return
    const timer = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(timer)
  }, [startedAt])
  if (startedAt == null) return null
  const total = Math.max(0, Math.floor((now - startedAt) / 1000))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0
    ? `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`
    : `${m}:${s.toString().padStart(2, "0")}`
}
