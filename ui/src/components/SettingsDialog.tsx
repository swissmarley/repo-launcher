import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import { api, toProblem } from '../lib/api'
import type { Account } from '../state/useAccount'
import type { Shell } from '../state/useShell'
import type { Updates } from '../state/useUpdates'
import { Button } from './Button'
import { Modal } from './Modal'
import { ProblemNotice } from './ProblemNotice'

/** Where people get new versions of builds that can't update themselves. */
const RELEASES_URL = 'https://github.com/swissmarley/repo-launcher/releases'

interface Props {
  open: boolean
  onClose: () => void
  account: Account
  shell: Shell
  updates: Updates
  /** Installs the available update (asking first if projects are running). */
  onInstallUpdate: () => void
  /** Opens the "Remove all Repo Launcher data" confirmation. */
  onRemoveAll: () => void
}

export function SettingsDialog({ open, onClose, account, shell, updates, onInstallUpdate, onRemoveAll }: Props) {
  const { t } = useTranslation()
  const s = account.settings
  const [aiKey, setAiKey] = useState('')
  const [problem, setProblem] = useState<Problem | null>(null)
  const [revokeUrl, setRevokeUrl] = useState<string | null>(null)

  const run = async (f: () => Promise<unknown>) => {
    setProblem(null)
    try {
      await f()
      await account.refresh()
    } catch (err) {
      setProblem(toProblem(err))
    }
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      wide
      title={t('settings.title')}
      footer={
        <Button variant="primary" onClick={onClose}>
          {t('action.done')}
        </Button>
      }
    >
      <div className="flex flex-col gap-6 text-sm">
        {s?.keychainProblem && <ProblemNotice problem={s.keychainProblem} />}
        {problem && <ProblemNotice problem={problem} onDismiss={() => setProblem(null)} />}

        <section aria-labelledby="settings-startup" className="flex flex-col gap-2">
          <h3 id="settings-startup" className="font-semibold">
            {t('settings.startup.heading')}
          </h3>
          <label className="flex items-start gap-2">
            <input
              type="checkbox"
              className="mt-1"
              checked={shell.settings?.launchAtLogin ?? false}
              onChange={(e) => run(() => shell.setLaunchAtLogin(e.target.checked))}
            />
            <span>
              {t('settings.startup.launchAtLogin')}
              <span className="block text-xs text-muted">
                {t('settings.startup.why', {
                  place: t(`settings.startup.place.${shell.settings?.platform ?? 'linux'}`, {
                    defaultValue: t('settings.startup.place.linux'),
                  }),
                })}
              </span>
            </span>
          </label>
        </section>

        <section aria-labelledby="settings-updates" className="flex flex-col gap-2">
          <h3 id="settings-updates" className="font-semibold">
            {t('updates.heading')}
          </h3>
          {updates.info && (
            <p className="text-muted">
              {t('updates.current', { version: updates.info.currentVersion })}
            </p>
          )}
          {updates.info && !updates.info.configured ? (
            <>
              <p className="text-muted">{t('updates.notConfigured')}</p>
              <div>
                <Button onClick={() => api.openExternal(RELEASES_URL)}>{t('updates.releases')}</Button>
              </div>
            </>
          ) : (
            <>
              {updates.problem && <ProblemNotice problem={updates.problem} onDismiss={updates.dismissProblem} />}
              <div className="flex flex-wrap items-center gap-2">
                {updates.available ? (
                  <>
                    <span role="status">{t('updates.ready', { version: updates.available })}</span>
                    <Button variant="primary" onClick={onInstallUpdate} disabled={updates.progress != null}>
                      {t('updates.install')}
                    </Button>
                  </>
                ) : (
                  <>
                    <Button onClick={() => void updates.check()} disabled={updates.checking}>
                      {updates.checking ? t('updates.checking') : t('updates.check')}
                    </Button>
                    {updates.upToDate && <span role="status">{t('update.upToDate')}</span>}
                  </>
                )}
              </div>
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  className="mt-1"
                  checked={updates.info?.autoCheck ?? true}
                  onChange={(e) => run(() => updates.setAutoCheck(e.target.checked))}
                />
                <span>
                  {t('updates.autoCheck')}
                  <span className="block text-xs text-muted">{t('updates.why')}</span>
                </span>
              </label>
            </>
          )}
        </section>

        <section aria-labelledby="settings-github" className="flex flex-col gap-2">
          <h3 id="settings-github" className="font-semibold">
            {t('settings.github.heading')}
          </h3>
          {account.account ? (
            <>
              <p>{t('settings.github.connectedAs', { login: account.account.login })}</p>
              <div className="flex flex-wrap gap-2">
                {s?.githubInstallUrl && (
                  <Button onClick={() => api.openExternal(s.githubInstallUrl!)}>{t('settings.github.chooseRepos')}</Button>
                )}
                <Button variant="ghost" onClick={() => run(async () => setRevokeUrl(await account.signOut()))}>
                  {t('settings.github.disconnect')}
                </Button>
              </div>
            </>
          ) : (
            <>
              <p className="text-muted">{t('settings.github.why')}</p>
              {revokeUrl && (
                <p className="text-muted">
                  {t('settings.github.disconnected')}{' '}
                  <button type="button" className="text-accent hover:underline" onClick={() => api.openExternal(revokeUrl)}>
                    {t('settings.github.revoke')}
                  </button>
                </p>
              )}
              <div>
                <Button variant="primary" onClick={() => account.connect()} disabled={!s?.githubConfigured}>
                  {t('github.connect')}
                </Button>
              </div>
              {s && !s.githubConfigured && <p className="text-xs text-muted">{t('problem.GITHUB_NOT_CONFIGURED.body')}</p>}
            </>
          )}
        </section>

        <section aria-labelledby="settings-trusted" className="flex flex-col gap-2">
          <h3 id="settings-trusted" className="font-semibold">
            {t('settings.trusted.heading')}
          </h3>
          <p className="text-muted">{t('settings.trusted.why')}</p>
          {s?.trustedPublishers.length ? (
            <ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
              {s.trustedPublishers.map((owner) => (
                <li key={owner} className="flex items-center justify-between px-3 py-2">
                  <span translate="no" className="font-mono">
                    {owner}
                  </span>
                  <Button variant="ghost" onClick={() => run(() => api.setTrustedPublisher(owner, false))}>
                    {t('settings.trusted.remove')}
                  </Button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-xs text-muted">{t('settings.trusted.none')}</p>
          )}
        </section>

        <section aria-labelledby="settings-ai" className="flex flex-col gap-2">
          <h3 id="settings-ai" className="font-semibold">
            {t('settings.ai.heading')}
          </h3>
          <label className="flex items-start gap-2">
            <input
              type="checkbox"
              className="mt-1"
              checked={s?.aiEnabled ?? false}
              onChange={(e) => run(() => api.setAiEnabled(e.target.checked))}
            />
            <span>
              {t('settings.ai.toggle')}
              <span className="block text-xs text-muted">{t('settings.ai.why')}</span>
            </span>
          </label>
          {s?.aiEnabled && (
            <form
              className="flex flex-col gap-1"
              onSubmit={(e) => {
                e.preventDefault()
                void run(async () => {
                  await api.setAiKey(aiKey)
                  setAiKey('')
                })
              }}
            >
              <label htmlFor="ai-key" className="font-medium">
                {t('keys.kind.apiKey', { service: 'Anthropic' })}
              </label>
              <div className="flex gap-2">
                <input
                  id="ai-key"
                  type="password"
                  autoComplete="off"
                  value={aiKey}
                  placeholder={s.aiHasKey ? t('keys.savedPlaceholder') : 'sk-ant-…'}
                  onChange={(e) => setAiKey(e.target.value)}
                  className="min-h-10 flex-1 rounded-lg border border-border bg-surface px-3 font-mono text-fg placeholder:font-sans placeholder:text-muted focus-visible:border-accent"
                />
                <Button type="submit" disabled={!aiKey.trim()}>
                  {t('keys.save')}
                </Button>
                {s.aiHasKey && (
                  <Button variant="ghost" onClick={() => run(() => api.setAiKey(null))}>
                    {t('keys.clear')}
                  </Button>
                )}
              </div>
              <button
                type="button"
                className="self-start text-xs text-accent hover:underline"
                onClick={() => api.openExternal('https://console.anthropic.com/settings/keys')}
              >
                {t('keys.whereToGet')}
              </button>
            </form>
          )}
        </section>

        <section aria-labelledby="settings-remove" className="flex flex-col gap-2 border-t border-border pt-5">
          <h3 id="settings-remove" className="font-semibold">
            {t('settings.removeAll.heading')}
          </h3>
          <p className="text-muted">{t('settings.removeAll.why')}</p>
          <div>
            <Button variant="danger" onClick={onRemoveAll}>
              {t('settings.removeAll.open')}
            </Button>
          </div>
        </section>
      </div>
    </Modal>
  )
}
