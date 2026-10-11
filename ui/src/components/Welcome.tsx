import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import type { Starter } from '../bindings/Starter'
import { api } from '../lib/api'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'
import { ProjectIcon } from './ProjectIcon'

interface Props {
  /** Adds the project and opens its review; resolves to a problem if adding failed. */
  onTry: (link: string) => Promise<Problem | null>
}

/**
 * The first screen (PLAN §11): no tour, one welcome line, the link field above, and a few
 * starter projects that need no keys or account, so the first install takes one click.
 */
export function Welcome({ onTry }: Props) {
  const { t } = useTranslation()
  const [starters, setStarters] = useState<Starter[]>([])
  const [busy, setBusy] = useState<string | null>(null)
  const [problem, setProblem] = useState<Problem | null>(null)

  useEffect(() => {
    let active = true
    api
      .starterProjects()
      .then((s) => active && setStarters(s))
      .catch(() => {})
    return () => {
      active = false
    }
  }, [])

  const tryStarter = async (s: Starter) => {
    setBusy(s.key)
    setProblem(null)
    const p = await onTry(s.link)
    setBusy(null)
    if (p) setProblem(p)
  }

  return (
    <section aria-labelledby="welcome-heading" className="motion-enter mx-auto mt-10 max-w-3xl">
      <div className="text-center">
        <h2 id="welcome-heading" className="text-2xl font-semibold">
          {t('empty.title')}
        </h2>
        <p className="mx-auto mt-3 max-w-xl text-muted">{t('empty.body')}</p>
      </div>

      {starters.length > 0 && (
        <div className="mt-10">
          <h3 id="starters-heading" className="text-base font-semibold">
            {t('starters.heading')}
          </h3>
          <p className="mt-1 text-sm text-muted">{t('starters.intro')}</p>
          {problem && (
            <div className="mt-3">
              <ProblemNotice problem={problem} onDismiss={() => setProblem(null)} />
            </div>
          )}
          <ul aria-labelledby="starters-heading" className="mt-4 grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-3">
            {starters.map((s, i) => {
              const name = t(`starters.items.${s.key}.name`)
              return (
                <li
                  key={s.key}
                  className="motion-enter flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 shadow-card"
                  style={{ animationDelay: `${i * 40}ms` }}
                >
                  <div className="flex items-start gap-3">
                    <ProjectIcon name={name} avatarUrl={s.avatarUrl || null} />
                    <div className="min-w-0">
                      <p className="font-semibold">{name}</p>
                      <p className="text-xs text-muted">{t(`kind.${s.kind}`)}</p>
                    </div>
                  </div>
                  <p className="flex-1 text-sm text-muted">{t(`starters.items.${s.key}.description`)}</p>
                  <Button
                    variant="secondary"
                    onClick={() => void tryStarter(s)}
                    disabled={busy != null}
                    aria-label={t('starters.tryNamed', { name })}
                  >
                    {busy === s.key ? t('add.adding') : t('starters.try')}
                  </Button>
                </li>
              )
            })}
          </ul>
          <p className="mt-4 text-center text-xs text-muted">{t('starters.safety')}</p>
        </div>
      )}
    </section>
  )
}
