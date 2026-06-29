import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { IconCalendarEvent, IconExternalLink, IconLoader2, IconRocket } from "@tabler/icons-react"
import { openUrl } from "@tauri-apps/plugin-opener"
import Markdown from "react-markdown"
import remarkGfm from "remark-gfm"

import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { formatDate } from "@/lib/format"

const RELEASES_API = "https://api.github.com/repos/radcolor-dev/radianite_val/releases/tags"
const RELEASES_URL = "https://github.com/radcolor-dev/radianite_val/releases/tag"

type ReleaseNotes = { version: string; body?: string | null; date?: string | null; url?: string | null }
type Props = { open: boolean; onOpenChange: (open: boolean) => void; release: ReleaseNotes | null; fetchFromGitHub?: boolean }
type GitHubRelease = { body?: string | null; published_at?: string | null; html_url?: string | null }

export function ReleaseNotesDialog({ open, onOpenChange, release, fetchFromGitHub = false }: Props) {
  const { t } = useTranslation()
  const [notes, setNotes] = useState<ReleaseNotes | null>(release)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(false)

  useEffect(() => {
    setNotes(release)
    setError(false)
    setLoading(false)
    if (!open || !release || !fetchFromGitHub) return

    const controller = new AbortController()
    const tag = release.version.startsWith("v") ? release.version : `v${release.version}`
    setLoading(true)
    fetch(`${RELEASES_API}/${encodeURIComponent(tag)}`, { headers: { Accept: "application/vnd.github+json" }, signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) throw new Error(`GitHub returned ${response.status}`)
        return response.json() as Promise<GitHubRelease>
      })
      .then((githubRelease) => setNotes({ ...release, body: githubRelease.body, date: githubRelease.published_at, url: githubRelease.html_url }))
      .catch((fetchError: unknown) => {
        if (!(fetchError instanceof DOMException && fetchError.name === "AbortError")) setError(true)
      })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [fetchFromGitHub, open, release?.version, release?.body, release?.date, release?.url])

  const version = notes?.version.replace(/^v/i, "")
  const releaseUrl = notes?.url ?? (version ? `${RELEASES_URL}/v${version}` : null)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[38rem] max-h-[calc(100%-2rem)] max-w-[calc(100%-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-4xl">
        <header className="flex items-start gap-3 border-b px-7 py-5 pe-14">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"><IconRocket className="size-5" /></span>
          <div className="min-w-0 flex-1">
            <DialogTitle className="text-base">{t("updates.releaseNotesTitle", { version })}</DialogTitle>
            <DialogDescription className="mt-1 flex items-center gap-1.5">
              {notes?.date ? <><IconCalendarEvent className="size-3.5" />{formatDate(new Date(notes.date))}</> : t("updates.releaseNotesDescription")}
            </DialogDescription>
          </div>
          {releaseUrl ? <Button variant="outline" size="sm" onClick={() => openUrl(releaseUrl)}><IconExternalLink data-icon="inline-start" />{t("updates.viewOnGitHub")}</Button> : null}
        </header>
        <ScrollArea className="min-h-0 flex-1">
          <div className="p-7">
            {loading ? <div className="flex h-52 items-center justify-center gap-2 text-muted-foreground"><IconLoader2 className="size-4 animate-spin" />{t("updates.loadingReleaseNotes")}</div> : null}
            {!loading && error ? <div className="flex h-52 items-center justify-center text-center text-muted-foreground">{t("updates.releaseNotesError")}</div> : null}
            {!loading && !error ? (
              <Markdown remarkPlugins={[remarkGfm]} components={{
                a: ({ href, children }) => <a href={href} className="text-primary underline underline-offset-3" onClick={(event) => { event.preventDefault(); if (href) void openUrl(href) }}>{children}</a>,
                h1: ({ children }) => <h1 className="mb-4 font-heading text-xl font-semibold">{children}</h1>,
                h2: ({ children }) => <h2 className="mt-7 mb-3 border-b pb-2 font-heading text-base font-semibold first:mt-0">{children}</h2>,
                h3: ({ children }) => <h3 className="mt-5 mb-2 font-heading text-sm font-semibold">{children}</h3>,
                p: ({ children }) => <p className="mb-3 text-sm/6 text-muted-foreground">{children}</p>,
                ul: ({ children }) => <ul className="mb-4 list-disc space-y-1 ps-5 text-sm/6 text-muted-foreground">{children}</ul>,
                ol: ({ children }) => <ol className="mb-4 list-decimal space-y-1 ps-5 text-sm/6 text-muted-foreground">{children}</ol>,
                code: ({ children }) => <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-foreground">{children}</code>,
                blockquote: ({ children }) => <blockquote className="mb-4 border-s-2 border-primary/50 ps-4 text-muted-foreground">{children}</blockquote>,
                hr: () => <hr className="my-6 border-border" />,
              }}>{notes?.body?.trim() || t("updates.noReleaseNotes")}</Markdown>
            ) : null}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  )
}
