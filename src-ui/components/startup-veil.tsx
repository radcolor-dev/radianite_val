import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"

import { AppIcon } from "@/components/app-icon"
import { cn } from "@/lib/utils"

export function StartupVeil({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    setVisible(active)
  }, [active])

  return (
    <div
      role="status"
      aria-live="polite"
      aria-hidden={!visible}
      className={cn("startup-veil", visible ? "startup-veil-visible" : "startup-veil-hidden")}
    >
      <div className="startup-glow" />
      <div className="startup-mark">
        <span className="startup-orbit" />
        <AppIcon className="relative size-14 rounded-xl shadow-2xl" />
      </div>
      <div className="text-center">
        <p className="text-lg font-semibold tracking-tight">{t("common.appName")}</p>
        <p className="mt-1 text-xs text-muted-foreground">{t("startup.preparing")}</p>
      </div>
      <div className="startup-progress" aria-hidden="true">
        <span />
      </div>
      <div className="startup-dots" aria-hidden="true">
        <span />
        <span />
        <span />
      </div>
    </div>
  )
}
