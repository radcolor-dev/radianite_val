import type { TFunction } from "i18next"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"

import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"

const SECOND = 1_000
const RECENT = 10 * SECOND
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

export function RelativeTime({
  date,
  fallback,
  coarse = false,
}: {
  date: Date | null
  fallback: string
  coarse?: boolean
}) {
  const { i18n, t } = useTranslation()
  const [now, setNow] = useState(Date.now)

  useEffect(() => {
    if (!date) return

    setNow(Date.now())
    const timer = window.setInterval(
      () => setNow(Date.now()),
      coarse ? 30 * SECOND : SECOND,
    )
    return () => window.clearInterval(timer)
  }, [coarse, date])

  if (!date) return fallback

  const locale = i18n.resolvedLanguage ?? i18n.language
  const exactTime = date.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  })

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} className="cursor-help rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-ring">
          {coarse
            ? formatCoarseRelativeTime(date, now, locale, t)
            : formatRelativeTime(date, now, locale)}
        </span>
      </TooltipTrigger>
      <TooltipContent>{exactTime}</TooltipContent>
    </Tooltip>
  )
}

function formatCoarseRelativeTime(
  date: Date,
  now: number,
  locale: string,
  t: TFunction,
) {
  const elapsed = Math.max(0, now - date.getTime())
  if (elapsed < MINUTE) return t("updates.relativeTime.justNow")
  if (elapsed < 2 * MINUTE) return t("updates.relativeTime.minuteAgo")
  if (elapsed < 6 * HOUR) return t("updates.relativeTime.whileAgo")

  const today = new Date(now)
  const checked = new Date(date)
  const todayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate())
  const checkedStart = new Date(checked.getFullYear(), checked.getMonth(), checked.getDate())
  const daysAgo = Math.round((todayStart.getTime() - checkedStart.getTime()) / DAY)

  if (daysAgo === 0) return t("updates.relativeTime.today")
  if (daysAgo === 1) return t("updates.relativeTime.yesterday")
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(-daysAgo, "day")
}

function formatRelativeTime(date: Date, now: number, locale: string) {
  const difference = date.getTime() - now
  const absoluteDifference = Math.abs(difference)
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" })

  // Normal polling runs every 2–5 seconds. Keep that healthy state visually
  // stable, then show real elapsed time once updates are actually delayed.
  if (absoluteDifference < RECENT) {
    return formatter.format(-1, "second")
  }
  if (absoluteDifference < MINUTE) {
    return formatter.format(Math.round(difference / SECOND), "second")
  }
  if (absoluteDifference < HOUR) {
    return formatter.format(Math.round(difference / MINUTE), "minute")
  }
  if (absoluteDifference < DAY) {
    return formatter.format(Math.round(difference / HOUR), "hour")
  }
  return formatter.format(Math.round(difference / DAY), "day")
}
