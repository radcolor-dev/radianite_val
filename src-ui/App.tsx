import { lazy, Suspense, useEffect, useState } from "react"

import { TitleBar } from "@/components/title-bar"
import { LiveMatchHero } from "@/components/live-match-hero"
import { CoreStatusCard } from "@/components/core-status-card"
import { OverlayCard } from "@/components/overlay-card"
import { DiscordCard } from "@/components/discord-card"
import { UpdatesCard } from "@/components/updates-card"
import { QuickInfoCard } from "@/components/quick-info-card"
import { StatusBar } from "@/components/status-bar"
import { StartupVeil } from "@/components/startup-veil"
import { useRadianite } from "@/lib/use-radianite"
import "./App.css"

const SettingsDialog = lazy(() =>
  import("@/components/settings-dialog").then((module) => ({
    default: module.SettingsDialog,
  })),
)

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }

  return (
    target.isContentEditable ||
    target.closest("input, textarea, select, [contenteditable='true']") !== null
  )
}

function isBlockedProductionShortcut(event: KeyboardEvent) {
  const key = event.key.toLowerCase()
  const modifier = event.ctrlKey || event.metaKey

  return (
    event.key === "F5" ||
    event.key === "F12" ||
    (modifier && key === "r") ||
    (modifier && key === "u") ||
    (modifier && event.shiftKey && ["c", "i", "j"].includes(key))
  )
}

function App() {
  const r = useRadianite()
  const [settingsOpen, setSettingsOpen] = useState(false)

  useEffect(() => {
    if (!import.meta.env.PROD) {
      return
    }

    document.documentElement.dataset.appHardened = "true"

    const preventContextMenu = (event: MouseEvent) => {
      event.preventDefault()
    }
    const preventDrag = (event: DragEvent) => {
      event.preventDefault()
    }
    const preventSelection = (event: Event) => {
      if (!isEditableTarget(event.target)) {
        event.preventDefault()
      }
    }
    const preventShortcuts = (event: KeyboardEvent) => {
      if (isBlockedProductionShortcut(event)) {
        event.preventDefault()
        event.stopPropagation()
      }
    }

    window.addEventListener("contextmenu", preventContextMenu)
    window.addEventListener("dragstart", preventDrag)
    window.addEventListener("selectstart", preventSelection)
    window.addEventListener("keydown", preventShortcuts, true)

    return () => {
      delete document.documentElement.dataset.appHardened
      window.removeEventListener("contextmenu", preventContextMenu)
      window.removeEventListener("dragstart", preventDrag)
      window.removeEventListener("selectstart", preventSelection)
      window.removeEventListener("keydown", preventShortcuts, true)
    }
  }, [])

  return (
    <div className="app-enter flex h-screen flex-col bg-background text-foreground">
      <TitleBar
        status={r.diagnostics.status}
        version={r.appVersion}
        busy={r.busy}
        onRefresh={r.refresh}
        onStartMonitor={r.startMonitor}
        onStopMonitor={r.stopMonitor}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <main className="flex-1 overflow-y-auto p-3">
        <div className="mx-auto flex w-full max-w-[1400px] flex-col gap-3">
          <div className="grid gap-3 lg:grid-cols-[minmax(0,1.6fr)_minmax(22rem,1fr)]">
            <LiveMatchHero snapshot={r.snapshot} presentation={r.presentation} />

            <div className="flex flex-col gap-3">
              <CoreStatusCard diagnostics={r.diagnostics} />
              <OverlayCard
                overlay={r.overlayStatus}
                onCopy={r.copyOverlayUrl}
                onOpen={r.openOverlayUrl}
              />
            </div>
          </div>

          <div className="grid gap-3 lg:grid-cols-3">
            <DiscordCard
              rpc={r.rpcStatus}
              snapshot={r.snapshot}
              presentation={r.presentation}
              busy={r.busy}
              onToggle={r.toggleRpc}
            />
            <UpdatesCard
              updater={r.updater}
              version={r.appVersion}
              canInstall={Boolean(r.availableUpdate)}
              lastChecked={r.lastChecked}
              onCheck={r.checkForUpdate}
              onInstall={r.installAvailableUpdate}
            />
            <QuickInfoCard
              overlay={r.overlayStatus}
              rpc={r.rpcStatus}
              snapshot={r.snapshot}
              lastSync={r.lastSync}
            />
          </div>
        </div>
      </main>

      <StatusBar
        status={r.diagnostics.status}
        lastSync={r.lastSync}
        uptimeMs={r.uptimeMs}
      />

      {settingsOpen ? (
        <Suspense fallback={null}>
          <SettingsDialog
            open
            onOpenChange={setSettingsOpen}
            settings={r.settings}
            onSetSetting={r.setSetting}
            overlay={r.overlayStatus}
            onCopyOverlay={r.copyOverlayUrl}
            onOpenOverlay={r.openOverlayUrl}
            busy={r.busy}
            appVersion={r.appVersion}
          />
        </Suspense>
      ) : null}

      <StartupVeil active={r.initializing} />
    </div>
  )
}

export default App
