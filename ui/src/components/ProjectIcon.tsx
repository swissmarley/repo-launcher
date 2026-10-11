import { useEffect, useState } from 'react'
import { api } from '../lib/api'

/** Icons already fetched, by project id and version (they change when the files do). */
const cache = new Map<string, string | null>()

interface Props {
  name: string
  avatarUrl: string | null
  /** Set for projects in the library: their own icon (from the repo) comes first. */
  projectId?: string
  /** Changes when the project's files may have changed (its status or update time). */
  version?: string
}

/** The project's own icon, else the owner's avatar, else a stable two-letter monogram. */
export function ProjectIcon({ name, avatarUrl, projectId, version = '' }: Props) {
  const cacheKey = projectId ? `${projectId}:${version}` : null
  const [fetched, setFetched] = useState<{ key: string; url: string | null } | null>(null)
  /** Images that failed to load; the next choice is shown instead. */
  const [failed, setFailed] = useState<string[]>([])

  useEffect(() => {
    if (!projectId || !cacheKey || cache.has(cacheKey)) return
    let active = true
    api
      .projectIcon(projectId)
      .then((url) => {
        cache.set(cacheKey, url)
        if (active) setFetched({ key: cacheKey, url })
      })
      .catch(() => {})
    return () => {
      active = false
    }
  }, [projectId, cacheKey])

  const own = cacheKey ? (cache.get(cacheKey) ?? (fetched?.key === cacheKey ? fetched.url : null)) : null
  const src = [own, avatarUrl].find((s): s is string => !!s && !failed.includes(s))
  if (src) {
    return (
      <img
        src={src}
        alt=""
        onError={() => setFailed((f) => [...f, src])}
        className="size-12 shrink-0 rounded-xl border border-border bg-surface object-contain"
      />
    )
  }
  const hue = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)
  const letters = name.replace(/[^a-zA-Z0-9]/g, '').slice(0, 2).toUpperCase() || '?'
  return (
    <div
      aria-hidden
      translate="no"
      className="grid size-12 shrink-0 place-items-center rounded-xl text-base font-semibold text-white"
      // OKLCH keeps the same lightness for every hue, so white letters stay readable (AA).
      style={{ background: `linear-gradient(135deg, oklch(0.5 0.13 ${hue}), oklch(0.42 0.13 ${(hue + 40) % 360}))` }}
    >
      {letters}
    </div>
  )
}
