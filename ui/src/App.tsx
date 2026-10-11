import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from './bindings/Problem'
import { AddBar } from './components/AddBar'
import { AiDialog, type AiTarget } from './components/AiDialog'
import { Button } from './components/Button'
import { KeysDialog } from './components/KeysDialog'
import { ProjectCard } from './components/ProjectCard'
import { SafetyDialog } from './components/SafetyDialog'
import { SettingsDialog } from './components/SettingsDialog'
import { SignInDialog } from './components/SignInDialog'
import { YourProjects } from './components/YourProjects'
import { api } from './lib/api'
import { useAccount } from './state/useAccount'
import { useProjects } from './state/useProjects'

type Tab = 'library' | 'yours'

export default function App() {
  const { t } = useTranslation()
  const projects = useProjects()
  const account = useAccount()
  const [tab, setTab] = useState<Tab>('library')
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [aiTarget, setAiTarget] = useState<AiTarget | null>(null)
  const empty = projects.loaded && projects.projects.length === 0
  const added = useMemo(() => new Set(projects.projects.map((p) => p.source)), [projects.projects])
  const showTabs = account.account != null

  /** Carries out a problem's suggested fix. Resolves true if the failed action should be retried. */
  const fix = useCallback(
    async (problem: Problem, projectId?: string): Promise<boolean> => {
      const f = problem.fix
      switch (f?.kind) {
        case 'connectGithub':
          return account.connect()
        case 'chooseRepositories':
        case 'openUrl':
          await api.openExternal(f.url).catch(() => {})
          return false
        case 'openSettings':
          setAiTarget(null)
          void account.refresh()
          setSettingsOpen(true)
          return false
        case 'askAi':
          if (projectId) setAiTarget({ projectId, name: projects.projects.find((p) => p.id === projectId)?.name ?? '' })
          return false
        case 'enterKeys':
          if (projectId) await projects.openKeys(projectId, true)
          return false
        case 'reviewSafety':
          if (projectId) await projects.install(projectId)
          return false
        default:
          if (projectId) await projects.retry(projectId)
          return false
      }
    },
    [account, projects],
  )

  return (
    <div className="flex h-full flex-col">
      <header className="border-b border-border bg-surface/80 px-6 py-4 backdrop-blur">
        <div className="mx-auto flex max-w-5xl flex-col gap-4">
          <div className="flex items-center gap-2">
            <h1 className="flex-1 text-lg font-semibold">{t('app.title')}</h1>
            {account.account ? (
              <span className="flex items-center gap-2 text-sm text-muted">
                {account.account.avatarUrl && <img src={account.account.avatarUrl} alt="" className="size-6 rounded-full" />}
                @{account.account.login}
              </span>
            ) : (
              account.settings?.githubConfigured && (
                <Button variant="ghost" onClick={() => account.connect()}>
                  {t('github.connect')}
                </Button>
              )
            )}
            <Button
              variant="ghost"
              onClick={() => {
                void account.refresh()
                setSettingsOpen(true)
              }}
            >
              {t('settings.open')}
            </Button>
          </div>
          <AddBar onAdd={projects.add} onFix={(p) => fix(p)} autoFocus={empty && tab === 'library'} />
          {showTabs && (
            <div role="tablist" aria-label={t('tabs.label')} className="flex gap-1">
              {(['library', 'yours'] as const).map((key) => (
                <button
                  key={key}
                  role="tab"
                  type="button"
                  aria-selected={tab === key}
                  onClick={() => setTab(key)}
                  className={`rounded-lg px-3 py-1.5 text-sm font-medium ${
                    tab === key ? 'bg-accent-soft text-fg' : 'text-muted hover:bg-sunken hover:text-fg'
                  }`}
                >
                  {t(`tabs.${key}`)}
                </button>
              ))}
            </div>
          )}
        </div>
      </header>

      <main className="flex-1 overflow-auto px-6 py-6">
        <div className="mx-auto max-w-5xl">
          {tab === 'yours' && showTabs ? (
            <YourProjects
              installUrl={account.settings?.githubInstallUrl ?? null}
              added={added}
              onAdd={async (input) => {
                const p = await projects.add(input)
                if (!p) setTab('library')
                return p
              }}
              onFix={(p) => void fix(p)}
            />
          ) : empty ? (
            <section className="mx-auto mt-16 max-w-md text-center">
              <h2 className="text-2xl font-semibold">{t('empty.title')}</h2>
              <p className="mt-3 text-muted">{t('empty.body')}</p>
            </section>
          ) : (
            <section aria-labelledby="library-heading">
              <h2 id="library-heading" className="sr-only">
                {t('library.heading')}
              </h2>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(20rem,1fr))] gap-4">
                {projects.projects.map((p) => (
                  <ProjectCard
                    key={p.id}
                    project={p}
                    live={projects.live[p.id]}
                    actions={projects}
                    onFix={(problem) => void fix(problem, p.id)}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      </main>

      <SafetyDialog gate={projects.gate} onApprove={projects.approveGate} onCancel={projects.closeGate} />
      <KeysDialog key={`keys-${projects.keys?.projectId ?? 'none'}`} form={projects.keys} onSave={projects.saveKeys} onClose={projects.closeKeys} />
      <SignInDialog signIn={account.signIn} onRetry={account.retrySignIn} onClose={account.closeSignIn} />
      <AiDialog
        key={`ai-${aiTarget?.projectId ?? 'none'}`}
        target={aiTarget}
        onClose={() => setAiTarget(null)}
        onReview={(id) => {
          setAiTarget(null)
          void projects.install(id)
        }}
        onFix={(p) => void fix(p, aiTarget?.projectId)}
      />
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} account={account} />
    </div>
  )
}
