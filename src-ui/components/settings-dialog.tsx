import { useState } from "react"
import { useTranslation } from "react-i18next"
import {
  IconBrandDiscord,
  IconBroadcast,
  IconCopy,
  IconExternalLink,
  IconHeart,
  IconInfoCircle,
  IconSettings,
  type Icon,
} from "@tabler/icons-react"
import { openUrl } from "@tauri-apps/plugin-opener"

import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { rpcLocales, uiLocales } from "@/lib/i18n"
import { cn } from "@/lib/utils"
import type { OverlayStatus, SettingKey, Settings } from "@/lib/types"

type TabId = "general" | "overlay" | "discord" | "donate" | "about"
const NAV: Array<{ id: TabId; key: string; icon: Icon }> = [
  { id: "general", key: "settings.nav.general", icon: IconSettings },
  { id: "overlay", key: "settings.nav.overlay", icon: IconBroadcast },
  { id: "discord", key: "settings.nav.discord", icon: IconBrandDiscord },
  { id: "donate", key: "settings.nav.donate", icon: IconHeart },
  { id: "about", key: "settings.nav.about", icon: IconInfoCircle },
]
const REPO_URL = "https://github.com/radcolor-dev/radianite_val"
const SITE_URL = "https://radcolor.dev"

function preventCloseWhileSelectOpen(event: Event) {
  const target = event.target
  if (target instanceof Element && target.closest('[data-slot="select-content"], [data-slot="select-trigger"]')) {
    event.preventDefault()
    return
  }
  if (document.querySelector('[data-slot="select-content"][data-open]')) {
    event.preventDefault()
  }
}

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  settings: Settings
  onSetSetting: <K extends SettingKey>(key: K, value: Settings[K]) => void
  overlay: OverlayStatus
  onCopyOverlay: () => void
  onOpenOverlay: () => void
  busy: boolean
  appVersion: string | null
}

export function SettingsDialog(props: Props) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<TabId>("general")
  const { open, onOpenChange, settings, onSetSetting, overlay, onCopyOverlay, onOpenOverlay, busy, appVersion } = props

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton
        className="flex h-[38rem] max-h-[calc(100%-2rem)] max-w-[calc(100%-2rem)] gap-0 overflow-hidden p-0 sm:max-w-4xl"
        onPointerDownOutside={preventCloseWhileSelectOpen}
        onInteractOutside={preventCloseWhileSelectOpen}
        onFocusOutside={preventCloseWhileSelectOpen}
      >
        <DialogTitle className="sr-only">{t("settings.title")}</DialogTitle>
        <DialogDescription className="sr-only">{t("settings.description")}</DialogDescription>
        <nav className="flex w-52 shrink-0 flex-col gap-1 border-e bg-sidebar/60 p-3">
          <span className="px-2.5 py-2 text-xs font-semibold tracking-[0.15em] text-muted-foreground uppercase">{t("settings.title")}</span>
          {NAV.map((item) => {
            const NavIcon = item.icon
            const active = item.id === activeTab
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setActiveTab(item.id)}
                className={cn("flex items-center gap-2.5 rounded-md px-2.5 py-2 text-start text-xs font-medium transition-colors", active ? "bg-sidebar-accent text-foreground" : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-foreground")}
              >
                <NavIcon className="size-4" />
                {t(item.key)}
              </button>
            )
          })}
        </nav>
        <div className="flex min-w-0 flex-1 flex-col">
          {activeTab === "donate" ? <DonatePanel className="page-transition" /> : (
            <ScrollArea key={activeTab} className="page-transition flex-1">
              <div className="flex flex-col gap-7 p-7">
                {activeTab === "general" ? <GeneralPanel settings={settings} onSetSetting={onSetSetting} /> : null}
                {activeTab === "overlay" ? <OverlayPanel overlay={overlay} onCopy={onCopyOverlay} onOpen={onOpenOverlay} /> : null}
                {activeTab === "discord" ? <DiscordPanel settings={settings} onSetSetting={onSetSetting} busy={busy} /> : null}
                {activeTab === "about" ? <AboutPanel version={appVersion} /> : null}
              </div>
            </ScrollArea>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}

function PanelHeading({ title, description }: { title: string; description: string }) {
  return <div className="flex flex-col gap-1.5"><h2 className="font-heading text-base font-medium">{title}</h2><p className="text-xs text-muted-foreground">{description}</p></div>
}

function SettingRow({ title, description, checked, onCheckedChange }: { title: string; description: string; checked: boolean; onCheckedChange: (checked: boolean) => void }) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4 rounded-lg border bg-background/40 px-4 py-3.5">
      <span className="flex flex-col gap-1"><span className="text-xs font-medium text-foreground">{title}</span><span className="text-xs text-muted-foreground">{description}</span></span>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </label>
  )
}

function LocaleRow({ title, description, value, kind, onChange, disabled }: { title: string; description: string; value: string; kind: "ui" | "rpc"; onChange: (value: string) => void; disabled?: boolean }) {
  const options = kind === "ui" ? uiLocales : rpcLocales
  return (
    <div className="flex items-center justify-between gap-4 rounded-lg border bg-background/40 px-4 py-3.5">
      <span className="flex flex-col gap-1"><span className="text-xs font-medium text-foreground">{title}</span><span className="text-xs text-muted-foreground">{description}</span></span>
      <Select value={value} disabled={disabled} onValueChange={onChange}>
        <SelectTrigger className="min-w-44" aria-label={title}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent position="popper" align="end">
          <SelectGroup>
            {options.map((locale) => <SelectItem key={locale.tag} value={locale.tag}>{locale.nativeName}</SelectItem>)}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  )
}

function GeneralPanel({ settings, onSetSetting }: Pick<Props, "settings" | "onSetSetting">) {
  const { t } = useTranslation()
  return <>
    <PanelHeading title={t("settings.nav.general")} description={t("settings.generalDescription")} />
    <div className="flex flex-col gap-3">
      <LocaleRow title={t("settings.appLanguage")} description={t("settings.appLanguageDescription")} value={settings.uiLocale} kind="ui" onChange={(value) => onSetSetting("uiLocale", value)} />
      <SettingRow title={t("settings.runAtBoot")} description={t("settings.runAtBootDescription")} checked={settings.runAtBoot} onCheckedChange={(value) => onSetSetting("runAtBoot", value)} />
      <SettingRow title={t("settings.minimizeToTray")} description={t("settings.minimizeToTrayDescription")} checked={settings.minimizeToTray} onCheckedChange={(value) => onSetSetting("minimizeToTray", value)} />
    </div>
  </>
}

function OverlayPanel({ overlay, onCopy, onOpen }: { overlay: OverlayStatus; onCopy: () => void; onOpen: () => void }) {
  const { t } = useTranslation()
  const url = overlay.url ?? null
  return <>
    <PanelHeading title={t("overlay.title")} description={t("settings.overlayDescription")} />
    <div className="flex flex-col gap-3">
      <div><p className="mb-1 text-xs text-muted-foreground">{t("overlay.sourceUrl")}</p><code className="block w-full truncate rounded-md border bg-background/60 px-2.5 py-1.5 font-mono text-xs">{url ?? t("overlay.notRunning")}</code></div>
      <div className="flex items-center gap-2"><Button variant="outline" onClick={onCopy} disabled={!url}><IconCopy data-icon="inline-start" />{t("overlay.copyUrl")}</Button><Button variant="outline" onClick={onOpen} disabled={!url}><IconExternalLink data-icon="inline-start" />{t("common.open")}</Button></div>
      <p className="text-xs text-muted-foreground">{t("overlay.suggestedSize")} <span className="font-mono text-foreground">360 × 90</span></p>
    </div>
  </>
}

function DiscordPanel({ settings, onSetSetting, busy }: Pick<Props, "settings" | "onSetSetting" | "busy">) {
  const { t } = useTranslation()
  return <>
    <PanelHeading title={t("settings.rpcTitle")} description={t("settings.rpcDescription")} />
    <div className="flex flex-col gap-3">
      <LocaleRow title={t("settings.rpcLanguage")} description={t("settings.rpcLanguageDescription")} value={settings.rpcLocale} kind="rpc" disabled={busy} onChange={(value) => onSetSetting("rpcLocale", value)} />
      <SettingRow
        title={t("settings.enableRpcOnStart")}
        description={t("settings.enableRpcOnStartDescription")}
        checked={settings.enableRpcOnStart}
        onCheckedChange={(value) => onSetSetting("enableRpcOnStart", value)}
      />
    </div>
  </>
}

function DonatePanel({ className }: { className?: string }) {
  const { t } = useTranslation()
  return <iframe title={t("settings.supportTitle")} src="https://radcolor.dev/donate" className={cn("size-full border-0 bg-background", className)} />
}

function AboutPanel({ version }: { version: string | null }) {
  const { t } = useTranslation()
  return <>
    <PanelHeading title={t("settings.aboutTitle")} description={t("settings.aboutDescription")} />
    <dl className="flex flex-col gap-2.5 text-xs">
      <div className="flex items-center justify-between rounded-lg border bg-background/40 px-4 py-3"><dt className="text-muted-foreground">{t("settings.version")}</dt><dd className="font-mono text-foreground">v{version ?? t("common.notAvailable")}</dd></div>
      <div className="flex items-center justify-between rounded-lg border bg-background/40 px-4 py-3"><dt className="text-muted-foreground">{t("settings.license")}</dt><dd className="font-mono text-foreground">GPL-3.0-only</dd></div>
    </dl>
    <div className="flex items-center gap-2"><Button variant="outline" onClick={() => openUrl(REPO_URL)}><IconExternalLink data-icon="inline-start" />{t("settings.repository")}</Button><Button variant="outline" onClick={() => openUrl(SITE_URL)}><IconExternalLink data-icon="inline-start" />{t("settings.website")}</Button></div>
  </>
}
