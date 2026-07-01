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
}: {
  date: Date | null
  fallback: string
}) {
  const { i18n } = useTranslation()
  const [now, setNow] = useState(Date.now)

  useEffect(() => {
    if (!date) return

    setNow(Date.now())
    const timer = window.setInterval(() => setNow(Date.now()), SECOND)
    return () => window.clearInterval(timer)
  }, [date])

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
          {formatRelativeTime(date, now, locale)}
        </span>
      </TooltipTrigger>
      <TooltipContent>{exactTime}</TooltipContent>
    </Tooltip>
  )
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
