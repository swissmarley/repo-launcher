import { useState } from 'react'

/** The owner's avatar when available, otherwise a stable two-letter monogram. */
export function ProjectIcon({ name, avatarUrl }: { name: string; avatarUrl: string | null }) {
  const [broken, setBroken] = useState(false)
  if (avatarUrl && !broken) {
    return (
      <img
        src={avatarUrl}
        alt=""
        onError={() => setBroken(true)}
        className="size-12 shrink-0 rounded-xl border border-border object-cover"
      />
    )
  }
  const hue = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)
  const letters = name.replace(/[^a-zA-Z0-9]/g, '').slice(0, 2).toUpperCase() || '?'
  return (
    <div
      aria-hidden
      className="grid size-12 shrink-0 place-items-center rounded-xl text-base font-semibold text-white"
      style={{ background: `linear-gradient(135deg, hsl(${hue} 55% 48%), hsl(${(hue + 40) % 360} 55% 38%))` }}
    >
      {letters}
    </div>
  )
}
