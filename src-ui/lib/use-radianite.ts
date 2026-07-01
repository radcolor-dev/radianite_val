import { useCallback, useEffect, useRef, useState } from "react"
import { convertFileSrc, invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getVersion } from "@tauri-apps/api/app"
import { openUrl } from "@tauri-apps/plugin-opener"
import { relaunch } from "@tauri-apps/plugin-process"
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater"
import { load, type Store } from "@tauri-apps/plugin-store"
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart"
import { toast } from "sonner"

import i18n, { applyUiLocale, detectedLocale, resolveLocale } from "@/lib/i18n"

import type {
  AppSnapshot,
  CoreStatus,
  DiagnosticSnapshot,
  LiveSnapshot,
  OverlayStatus,
  RpcStatus,
  SettingKey,
  Settings,
  UpdaterState,
  LocalizedMessage,
  ValorantPresentation,
} from "@/lib/types"

const localizedMessage = (
  key: string,
  args?: Record<string, string | number>,
  detail?: string,
): LocalizedMessage => ({ key, args, detail })

const initialStatus: CoreStatus = {
  kind: "disconnected",
  message: localizedMessage("status.message.notStarted"),
  monitored: false,
  updatedAt: "",
}

const initialDiagnostics: DiagnosticSnapshot = {
  status: initialStatus,
  riotInstallsJsonExists: false,
  lockfileExists: false,
  lockfilePortPresent: false,
  localApiReady: false,
  sessionProductIds: [],
  valorantSessionPresent: false,
  puuidPresent: false,
  accessTokenReady: false,
  entitlementTokenReady: false,
  updatedAt: "",
}

const initialRpcStatus: RpcStatus = {
  enabled: false,
  connected: false,
  configured: false,
  message: localizedMessage("status.rpc.notLoaded"),
  locale: "en-US",
  preview: null,
  updatedAt: "",
}

const initialOverlayStatus: OverlayStatus = {
  enabled: false,
  url: null,
  port: null,
  message: localizedMessage("status.overlay.notLoaded"),
  updatedAt: "",
}

const initialUpdaterState: UpdaterState = {
  status: "idle",
  message: localizedMessage("updates.state.idle"),
  progress: null,
}

const defaultSettings: Settings = {
  runAtBoot: false,
  minimizeToTray: true,
  enableRpcOnStart: true,
  uiLocale: detectedLocale("ui"),
  rpcLocale: detectedLocale("rpc"),
}

const SETTINGS_STORE = "settings.json"

function presentationAssetUrl(value?: string | null) {
  if (!value || /^(?:https?:|data:)/i.test(value)) return value
  return convertFileSrc(value)
}

function localizePresentationAssets(
  presentation: ValorantPresentation,
): ValorantPresentation {
  return {
    ...presentation,
    agentIconUrl: presentationAssetUrl(presentation.agentIconUrl),
    agentPortraitUrl: presentationAssetUrl(presentation.agentPortraitUrl),
    mapSplashUrl: presentationAssetUrl(presentation.mapSplashUrl),
    mapListViewIconUrl: presentationAssetUrl(presentation.mapListViewIconUrl),
    rankIconUrl: presentationAssetUrl(presentation.rankIconUrl),
  }
}

export function useRadianite() {
  const [diagnostics, setDiagnostics] =
    useState<DiagnosticSnapshot>(initialDiagnostics)
  const [snapshot, setSnapshot] = useState<LiveSnapshot | null>(null)
  const [presentation, setPresentation] = useState<ValorantPresentation | null>(null)
  const [rpcStatus, setRpcStatus] = useState<RpcStatus>(initialRpcStatus)
  const [overlayStatus, setOverlayStatus] =
    useState<OverlayStatus>(initialOverlayStatus)
  const [updater, setUpdater] = useState<UpdaterState>(initialUpdaterState)
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null)
  const [busy, setBusy] = useState(false)
  const [appVersion, setAppVersion] = useState<string | null>(null)
  const [lastSync, setLastSync] = useState<Date | null>(null)
  const [lastChecked, setLastChecked] = useState<Date | null>(null)
  const [startedAt] = useState<number>(() => Date.now())
  const [settings, setSettings] = useState<Settings>(defaultSettings)
  const [backendReady, setBackendReady] = useState(false)
  const [settingsReady, setSettingsReady] = useState(false)
  const settingsStore = useRef<Store | null>(null)

  const refresh = useCallback(async () => {
    const next = await invoke<AppSnapshot>("app_get_snapshot")

    setDiagnostics(next.diagnostics)
    setSnapshot(next.liveSnapshot)
    setRpcStatus(next.rpcStatus)
    setOverlayStatus(next.overlayStatus)
    setLastSync(new Date())
  }, [])

  const runCommand = useCallback(
    async (operation: () => Promise<void>) => {
      setBusy(true)
      try {
        await operation()
        await refresh()
      } catch (err) {
        toast.error(errorText(err))
      } finally {
        setBusy(false)
      }
    },
    [refresh],
  )

  const startMonitor = useCallback(
    () =>
      runCommand(async () => {
        await invoke<CoreStatus>("riot_start_monitor")
      }),
    [runCommand],
  )

  const stopMonitor = useCallback(
    () =>
      runCommand(async () => {
        await invoke<CoreStatus>("riot_stop_monitor")
      }),
    [runCommand],
  )

  const toggleRpc = useCallback(
    () =>
      runCommand(async () => {
        await invoke<RpcStatus>("discord_rpc_set_enabled", {
          enabled: !rpcStatus.enabled,
        })
      }),
    [runCommand, rpcStatus.enabled],
  )

  const copyOverlayUrl = useCallback(async () => {
    if (!overlayStatus.url) return
    try {
      await navigator.clipboard.writeText(overlayStatus.url)
      toast.success(i18n.t("overlay.copied"))
    } catch (err) {
      toast.error(errorText(err))
    }
  }, [overlayStatus.url])

  const openOverlayUrl = useCallback(async () => {
    if (!overlayStatus.url) return
    try {
      await openUrl(overlayStatus.url)
    } catch (err) {
      toast.error(errorText(err))
    }
  }, [overlayStatus.url])

  const setSetting = useCallback(
    async <K extends SettingKey>(key: K, value: Settings[K]) => {
      const previous = settings[key]
      const changesLocale = key === "uiLocale" || key === "rpcLocale"
      if (changesLocale) setBusy(true)
      setSettings((current) => ({ ...current, [key]: value }))

      try {
        if (key === "runAtBoot") {
          if (value) {
            if (!(await isAutostartEnabled())) await enableAutostart()
          } else {
            if (await isAutostartEnabled()) await disableAutostart()
          }
        }

        if (key === "uiLocale" && typeof value === "string") {
          const locale = await applyUiLocale(value)
          await invoke("localization_set_ui_locale", { locale })
          value = locale as Settings[K]
          setSettings((current) => ({ ...current, uiLocale: locale }))
        }

        if (key === "rpcLocale" && typeof value === "string") {
          const locale = resolveLocale([value], "rpc")
          const status = await invoke<RpcStatus>("discord_rpc_set_locale", { locale })
          setRpcStatus(status)
          value = locale as Settings[K]
          setSettings((current) => ({ ...current, rpcLocale: locale }))
        }

        const store = settingsStore.current
        if (store) {
          await store.set(key, value)
          await store.save()
        }
      } catch (err) {
        setSettings((current) => ({ ...current, [key]: previous }))
        if (key === "uiLocale" && typeof previous === "string") void applyUiLocale(previous)
        toast.error(errorText(err))
      } finally {
        if (changesLocale) setBusy(false)
      }
    },
    [settings],
  )

  const checkForUpdate = useCallback(async () => {
    setUpdater((current) => ({
      ...current,
      status: "checking",
      message: localizedMessage("updates.checking"),
      progress: null,
    }))

    try {
      const update = await check()
      setAvailableUpdate(update)
      setLastChecked(new Date())

      if (!update) {
        setUpdater((current) => ({
          ...current,
          status: "current",
          message: localizedMessage("updates.current"),
          progress: null,
        }))
        return
      }

      setUpdater({
        status: "available",
        message: localizedMessage("updates.available", { version: update.version }),
        currentVersion: update.currentVersion,
        version: update.version,
        date: update.date,
        body: update.body,
        progress: null,
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setUpdater((current) => ({ ...current, status: "error", message: localizedMessage("errors.generic", undefined, message) }))
      setLastChecked(new Date())
      toast.error(message)
    }
  }, [])

  const installAvailableUpdate = useCallback(async () => {
    if (!availableUpdate) return

    setUpdater((current) => ({
      ...current,
      status: "installing",
      message: localizedMessage("updates.installing", { version: availableUpdate.version }),
      progress: 0,
    }))

    let downloadedBytes = 0
    let contentLength: number | undefined
    const onDownloadEvent = (event: DownloadEvent) => {
      if (event.event === "Started") {
        downloadedBytes = 0
        contentLength = event.data.contentLength
        setUpdater((current) => ({
          ...current,
          message: contentLength
            ? localizedMessage("updates.downloadingSize", { size: formatBytes(contentLength) })
            : localizedMessage("updates.downloading"),
          progress: 0,
        }))
        return
      }

      if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength
        setUpdater((current) => ({
          ...current,
          message: contentLength
            ? localizedMessage("updates.downloadedOf", { downloaded: formatBytes(downloadedBytes), total: formatBytes(contentLength) })
            : localizedMessage("updates.downloaded", { downloaded: formatBytes(downloadedBytes) }),
          progress: contentLength
            ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
            : null,
        }))
        return
      }

      setUpdater((current) => ({
        ...current,
        message: localizedMessage("updates.installingNow"),
        progress: 100,
      }))
    }

    try {
      await availableUpdate.downloadAndInstall(onDownloadEvent)
      setUpdater((current) => ({
        ...current,
        status: "installed",
        message: localizedMessage("updates.installed"),
        progress: 100,
      }))
      await relaunch()
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setUpdater((current) => ({ ...current, status: "error", message: localizedMessage("errors.generic", undefined, message) }))
      toast.error(message)
    }
  }, [availableUpdate])

  useEffect(() => {
    let active = true
    const unlistenCallbacks: Array<() => void> = []

    const initializeBackend = async () => {
      try {
        const listeners = await Promise.all([
          listen<CoreStatus>("riot:status", (event) => {
            setDiagnostics((current) => ({ ...current, status: event.payload }))
          }),
          listen<LiveSnapshot | null>("riot:snapshot", (event) => {
            setSnapshot(event.payload)
            setLastSync(new Date())
          }),
          listen<RpcStatus>("discord:status", (event) => {
            setRpcStatus(event.payload)
          }),
        ])

        if (!active) {
          listeners.forEach((unlisten) => unlisten())
          return
        }
        unlistenCallbacks.push(...listeners)

        void getVersion()
          .then((version) => { if (active) setAppVersion(version) })
          .catch(() => { if (active) setAppVersion(null) })

        const status = await invoke<CoreStatus>("riot_start_monitor")
        if (active) {
          setDiagnostics((current) => ({ ...current, status }))
          setBackendReady(true)
          void refresh().catch((err) => {
            if (active) toast.error(errorText(err))
          })
        }
      } catch (err) {
        if (active) toast.error(errorText(err))
      } finally {
        if (active) setBackendReady(true)
      }
    }

    void initializeBackend()

    return () => {
      active = false
      unlistenCallbacks.forEach((unlisten) => unlisten())
    }
  }, [refresh])

  useEffect(() => {
    let active = true

    const loadSettings = async () => {
      try {
        const store = await load(SETTINGS_STORE)
        settingsStore.current = store

        const [storedRunAtBoot, storedMinimizeToTray, storedEnableRpc, storedUiLocale, storedRpcLocale] =
          await Promise.all([
            store.get<boolean>("runAtBoot"),
            store.get<boolean>("minimizeToTray"),
            store.get<boolean>("enableRpcOnStart"),
            store.get<string>("uiLocale"),
            store.get<string>("rpcLocale"),
          ])
        const runAtBoot = storedRunAtBoot ?? defaultSettings.runAtBoot
        const minimizeToTray = storedMinimizeToTray ?? defaultSettings.minimizeToTray
        const enableRpcOnStart = storedEnableRpc ?? defaultSettings.enableRpcOnStart
        const uiLocale = resolveLocale(
          [storedUiLocale ?? defaultSettings.uiLocale],
          "ui",
        )
        const rpcLocale = resolveLocale(
          [storedRpcLocale ?? defaultSettings.rpcLocale],
          "rpc",
        )

        const autostartActive = await isAutostartEnabled().catch(
          () => runAtBoot,
        )

        await applyUiLocale(uiLocale)
        if (active) {
          setSettings({
            runAtBoot: autostartActive,
            minimizeToTray,
            enableRpcOnStart,
            uiLocale,
            rpcLocale,
          })
          setSettingsReady(true)
        }

        void Promise.all([
          invoke("localization_set_ui_locale", { locale: uiLocale }),
          (async () => {
            await invoke<RpcStatus>("discord_rpc_set_locale", { locale: rpcLocale })
            return invoke<RpcStatus>("discord_rpc_set_enabled", {
              enabled: enableRpcOnStart,
            })
          })(),
        ]).then(([, rpc]) => {
          if (active) setRpcStatus(rpc)
        }).catch((err) => {
          if (active) toast.error(errorText(err))
        })

        await Promise.all([
          store.set("runAtBoot", autostartActive),
          store.set("uiLocale", uiLocale),
          store.set("rpcLocale", rpcLocale),
          store.set("enableRpcOnStart", enableRpcOnStart),
        ])
        await store.save()
      } catch (err) {
        if (active) toast.error(errorText(err))
      } finally {
        if (active) setSettingsReady(true)
      }
    }

    loadSettings()

    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    let active = true
    if (!snapshot) {
      setPresentation(null)
      return
    }
    setPresentation(null)

    invoke<ValorantPresentation>("valorant_get_presentation", {
      locale: settings.uiLocale,
      agentId: snapshot.agentId,
      mapId: snapshot.mapId,
      tier: snapshot.rank?.tier,
    })
      .then((nextPresentation) => {
        if (active) setPresentation(localizePresentationAssets(nextPresentation))
      })
      .catch(() => {
        if (active) setPresentation(null)
      })

    return () => {
      active = false
    }
  }, [snapshot?.agentId, snapshot?.mapId, snapshot?.rank?.tier, settings.uiLocale])

  return {
    diagnostics,
    snapshot,
    presentation,
    rpcStatus,
    overlayStatus,
    updater,
    availableUpdate,
    busy,
    appVersion,
    lastSync,
    lastChecked,
    startedAt,
    settings,
    initializing: !backendReady || !settingsReady,
    setSetting,
    refresh: () => runCommand(refresh),
    startMonitor,
    stopMonitor,
    toggleRpc,
    copyOverlayUrl,
    openOverlayUrl,
    checkForUpdate,
    installAvailableUpdate,
  }
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`
  const kb = value / 1024
  if (kb < 1024) return `${formatDecimal(kb)} KB`
  const mb = kb / 1024
  return `${formatDecimal(mb)} MB`
}

function formatDecimal(value: number) {
  return new Intl.NumberFormat(i18n.language, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value)
}

function errorText(error: unknown) {
  const detail = error instanceof Error ? error.message : String(error)
  return i18n.t("errors.withDetail", {
    message: i18n.t("errors.generic"),
    detail,
  })
}
