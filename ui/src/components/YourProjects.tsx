import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import type { RemoteRepo } from '../bindings/RemoteRepo'
import { api, toProblem } from '../lib/api'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'

interface Props {
  installUrl: string | null
  /** Sources already in the library (`github:owner/repo`), to show "Added". */
  added: Set<string>
  onAdd: (input: string) => Promise<Problem | null>
  onFix: (p: Problem) => void
}

/** Repositories the user shared with Repo Launcher on GitHub. */
export function YourProjects({ installUrl, added, onAdd, onFix }: Props) {
  const { t } = useTranslation()
  const [repos, setRepos] = useState<RemoteRepo[] | null>(null)
  const [problem, setProblem] = useState<Problem | null>(null)
  const [adding, setAdding] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    api
      .githubRepositories()
      .then((r) => active && setRepos(r))
      .catch((err) => active && setProblem(toProblem(err)))
    return () => {
      active = false
    }
  }, [])

  const add = async (fullName: string) => {
    setAdding(fullName)
    const p = await onAdd(`https://github.com/${fullName}`)
    setAdding(null)
    if (p) setProblem(p)
  }

  return (
    <section aria-labelledby="yours-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 id="yours-heading" className="text-base font-semibold">
          {t('yours.heading')}
        </h2>
        {installUrl && (
          <Button variant="ghost" onClick={() => api.openExternal(installUrl)}>
            {t('yours.chooseRepos')}
          </Button>
        )}
      </div>
      {problem && <ProblemNotice problem={problem} onFix={() => onFix(problem)} onDismiss={() => setProblem(null)} />}
      {repos == null && !problem && <p className="text-sm text-muted">{t('yours.loading')}</p>}
      {repos?.length === 0 && <p className="text-sm text-muted">{t('yours.empty')}</p>}
      {repos && repos.length > 0 && (
        <ul className="flex flex-col divide-y divide-border rounded-xl border border-border bg-surface">
          {repos.map((r) => {
            const isAdded = added.has(`github:${r.fullName.toLowerCase()}`)
            return (
              <li key={r.fullName} className="flex items-center gap-4 px-4 py-3">
                <div className="min-w-0 flex-1">
                  <p className="flex items-center gap-2 truncate font-medium">
                    {r.fullName}
                    {r.private && (
                      <span className="rounded-full border border-border px-2 text-xs font-normal text-muted">
                        {t('project.private')}
                      </span>
                    )}
                  </p>
                  {r.description && <p className="truncate text-sm text-muted">{r.description}</p>}
                </div>
                <Button onClick={() => add(r.fullName)} disabled={isAdded || adding === r.fullName}>
                  {isAdded ? t('yours.added') : adding === r.fullName ? t('add.adding') : t('add.button')}
                </Button>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
