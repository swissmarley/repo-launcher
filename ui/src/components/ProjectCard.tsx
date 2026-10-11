import { DropdownMenu } from 'radix-ui'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import type { ProjectView } from '../lib/api'
import type { Live, Projects } from '../state/useProjects'
import { ActivitySheet } from './ActivitySheet'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'
import { ProgressBar } from './ProgressBar'
import { ProjectIcon } from './ProjectIcon'
import { RemoveDialog } from './RemoveDialog'

type StatusKey = 'added' | 'installing' | 'installed' | 'needsAttention' | 'starting' | 'running'

const dotColor: Record<StatusKey, string> = {
  added: 'bg-muted',
  installing: 'bg-accent',
  installed: 'bg-success',
  needsAttention: 'bg-warning',
  starting: 'bg-accent animate-pulse',
  running: 'bg-success',
}

function statusOf(p: ProjectView, live: Live | undefined): StatusKey {
  if (live?.busy === 'review' || live?.busy === 'install' || live?.busy === 'update' || p.status === 'installing') return 'installing'
  if (live?.busy === 'launch' || live?.runState === 'starting') return 'starting'
  if (p.running) return 'running'
  return p.status
}

function compact(n: number) {
  return new Intl.NumberFormat(undefined, { notation: 'compact' }).format(n)
}

export function ProjectCard({
  project: p,
  live,
  actions,
  platform,
  onFix,
  onAppStartsAtLogin,
}: {
  project: ProjectView
  live?: Live
  actions: Projects
  /** `macos`, `windows` or `linux`, for wording. */
  platform: string
  onFix: (p: Problem) => void
  /** Repo Launcher itself was set to start at login, so this project can start. */
  onAppStartsAtLogin: () => void
}) {
  const { t } = useTranslation()
  const [activityOpen, setActivityOpen] = useState(false)
  const [removeOpen, setRemoveOpen] = useState(false)
  const status = statusOf(p, live)
  const installing = status === 'installing'
  const progress = live?.progress

  const stepText =
    live?.busy === 'review'
      ? t('stage.reviewing')
      : progress?.label
        ? t(`step.${progress.label.kind}`, progress.label as Record<string, string>)
        : progress
          ? t(`stage.${progress.stage}`)
          : t('stage.downloading')

  const meta = [p.owner, p.isPrivate ? t('project.private') : null, p.stars != null ? `★ ${compact(p.stars)}` : null, p.license]
    .filter(Boolean)
    .join(' · ')

  return (
    <article
      aria-labelledby={`name-${p.id}`}
      className="flex flex-col gap-4 rounded-xl border border-border bg-surface p-5 shadow-card"
    >
      <header className="flex items-start gap-4">
        <ProjectIcon name={p.name} avatarUrl={p.avatarUrl} />
        <div className="min-w-0 flex-1">
          <h3 id={`name-${p.id}`} className="truncate text-base font-semibold">
            {p.name}
          </h3>
          {meta && <p className="truncate text-sm text-muted">{meta}</p>}
        </div>
        <span className="flex shrink-0 items-center gap-2 text-sm text-muted">
          <span aria-hidden className={`size-2.5 rounded-full ${dotColor[status]}`} />
          {t(`status.${status}`)}
        </span>
      </header>

      {p.description && <p className="line-clamp-2 text-sm text-muted">{p.description}</p>}
      {p.kind && !installing && <p className="text-xs text-muted">{t(`kind.${p.kind}`)}</p>}

      {installing && (
        <div className="flex flex-col gap-2" aria-live="polite">
          <p className="text-sm text-fg">{stepText}</p>
          <ProgressBar value={progress?.fraction ?? 0.02} label={stepText} />
        </div>
      )}

      {live?.notice && (
        <p role="status" className="flex items-center justify-between rounded-lg bg-accent-soft px-3 py-2 text-sm text-fg">
          {t(live.notice.key, live.notice.params)}
          <button type="button" className="text-muted hover:text-fg" onClick={() => actions.dismissNotice(p.id)}>
            {t('problem.dismiss')}
          </button>
        </p>
      )}

      {live?.problem && (
        <ProblemNotice
          problem={live.problem}
          onFix={() => live.problem && onFix(live.problem)}
          onDismiss={() => actions.dismissProblem(p.id)}
        />
      )}

      <footer className="mt-auto flex items-center gap-2">
        <PrimaryAction project={p} status={status} live={live} actions={actions} />
        {p.running && p.url && (
          <Button onClick={() => actions.stop(p.id)} disabled={live?.busy === 'stop'}>
            {t('action.stop')}
          </Button>
        )}
        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <Button variant="ghost" className="ml-auto" aria-label={t('action.more', { name: p.name })}>
              <span aria-hidden className="text-lg leading-none">
                ⋯
              </span>
            </Button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content
              align="end"
              sideOffset={4}
              className="min-w-44 rounded-lg border border-border bg-surface p-1 text-sm shadow-card"
            >
              {p.kind === 'terminal' && !p.useSystemTerminal && (
                <DropdownMenu.Item onSelect={() => actions.openConsole(p.id)} className="cursor-default rounded-md px-3 py-2 outline-none data-disabled:opacity-50 data-highlighted:bg-sunken">
                  {t('action.openConsole')}
                </DropdownMenu.Item>
              )}
              {p.owner && (
                <DropdownMenu.Item disabled={installing} onSelect={() => actions.update(p.id)} className="cursor-default rounded-md px-3 py-2 outline-none data-disabled:opacity-50 data-highlighted:bg-sunken">
                  {t('action.checkUpdates')}
                </DropdownMenu.Item>
              )}
              {p.status !== 'added' && (
                <DropdownMenu.Item onSelect={() => actions.openKeys(p.id, false)} className="cursor-default rounded-md px-3 py-2 outline-none data-disabled:opacity-50 data-highlighted:bg-sunken">
                  {t('action.keys')}
                </DropdownMenu.Item>
              )}
              {p.status !== 'added' && (
                <DropdownMenu.Item disabled={installing} onSelect={() => actions.repair(p.id)} className="cursor-default rounded-md px-3 py-2 outline-none data-disabled:opacity-50 data-highlighted:bg-sunken">
                  {t('action.repair')}
                </DropdownMenu.Item>
              )}
              {p.status === 'installed' && (
                <>
                  <DropdownMenu.Separator className="my-1 h-px bg-border" />
                  <DropdownMenu.Item onSelect={() => actions.toggleShortcut(p.id, platform)} className="cursor-default rounded-md px-3 py-2 outline-none data-disabled:opacity-50 data-highlighted:bg-sunken">
                    {p.shortcuts.length ? t('action.removeShortcut') : t('action.createShortcut')}
                  </DropdownMenu.Item>
                  <DropdownMenu.CheckboxItem
                    checked={p.startAtLogin}
                    onSelect={() => actions.toggleStartAtLogin(p.id, onAppStartsAtLogin)}
                    className="flex cursor-default items-center gap-2 rounded-md px-3 py-2 outline-none data-highlighted:bg-sunken"
                  >
                    {t('action.startAtLogin')}
                    <CheckMark checked={p.startAtLogin} />
                  </DropdownMenu.CheckboxItem>
                  {p.kind === 'terminal' && (
                    <DropdownMenu.CheckboxItem
                      checked={p.useSystemTerminal}
                      onSelect={() => actions.toggleSystemTerminal(p.id)}
                      className="flex cursor-default items-center gap-2 rounded-md px-3 py-2 outline-none data-highlighted:bg-sunken"
                    >
                      {t('action.useSystemTerminal')}
                      <CheckMark checked={p.useSystemTerminal} />
                    </DropdownMenu.CheckboxItem>
                  )}
                </>
              )}
              <DropdownMenu.Separator className="my-1 h-px bg-border" />
              <DropdownMenu.Item
                onSelect={() => setActivityOpen(true)}
                className="cursor-default rounded-md px-3 py-2 outline-none data-highlighted:bg-sunken"
              >
                {t('action.showActivity')}
              </DropdownMenu.Item>
              <DropdownMenu.Item
                disabled={installing}
                onSelect={() => setRemoveOpen(true)}
                className="cursor-default rounded-md px-3 py-2 text-danger outline-none data-disabled:opacity-50 data-highlighted:bg-danger-soft"
              >
                {t('action.remove')}
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      </footer>

      <ActivitySheet projectId={p.id} name={p.name} open={activityOpen} onOpenChange={setActivityOpen} />
      <RemoveDialog name={p.name} open={removeOpen} onOpenChange={setRemoveOpen} onConfirm={() => actions.remove(p.id)} />
    </article>
  )
}

/** A checkbox menu item's mark, at the end so labels line up with the other items. */
function CheckMark({ checked }: { checked: boolean }) {
  return (
    <span aria-hidden className="ml-auto inline-block w-3 pl-3 text-accent">
      {checked ? '✓' : ''}
    </span>
  )
}

function PrimaryAction({
  project: p,
  status,
  live,
  actions,
}: {
  project: ProjectView
  status: StatusKey
  live?: Live
  actions: Projects
}) {
  const { t } = useTranslation()
  switch (status) {
    case 'installing':
      return (
        <Button onClick={() => actions.cancel(p.id)} disabled={live?.busy === 'review' && !live.progress}>
          {t('action.cancel')}
        </Button>
      )
    case 'added':
    case 'needsAttention':
      return (
        <Button variant="primary" size="lg" onClick={() => actions.install(p.id)}>
          {t('action.install')}
        </Button>
      )
    case 'starting':
      return (
        <Button variant="primary" size="lg" disabled>
          {t('status.starting')}…
        </Button>
      )
    case 'running':
      return p.url ? (
        <Button variant="primary" size="lg" onClick={() => actions.open(p.id)}>
          {t('action.open')}
        </Button>
      ) : (
        <Button size="lg" onClick={() => actions.stop(p.id)} disabled={live?.busy === 'stop'}>
          {t('action.stop')}
        </Button>
      )
    case 'installed':
      return (
        <Button variant="primary" size="lg" onClick={() => actions.launch(p.id)}>
          {t('action.launch')}
        </Button>
      )
  }
}
