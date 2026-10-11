import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { RiskSignal } from '../bindings/RiskSignal'
import type { SummaryItem } from '../bindings/SummaryItem'
import type { Gate } from '../state/useProjects'
import { Button } from './Button'
import { Modal } from './Modal'

interface Props {
  gate: Gate | null
  onApprove: (trustOwner: boolean) => void
  onCancel: () => void
}

function summaryText(t: ReturnType<typeof useTranslation>['t'], item: SummaryItem) {
  switch (item.kind) {
    case 'download':
      return t(item.private ? 'safety.summary.downloadPrivate' : 'safety.summary.download')
    case 'needsKeys':
      return t('safety.summary.needsKeys', { names: item.names.join(', ') })
    default:
      return t(`safety.summary.${item.kind}`, item as Record<string, unknown>)
  }
}

function Signal({ signal }: { signal: RiskSignal }) {
  const { t } = useTranslation()
  const red = signal.level === 'red'
  const params = signal.params as Record<string, string>
  return (
    <li className={`rounded-lg border p-3 text-sm ${red ? 'border-danger/40 bg-danger-soft' : 'border-warning/40 bg-sunken'}`}>
      <p className="flex items-start gap-2 font-medium">
        <span aria-hidden className={`mt-1.5 size-2 shrink-0 rounded-full ${red ? 'bg-danger' : 'bg-warning'}`} />
        <span>
          <span className="sr-only">{t(red ? 'safety.level.red' : 'safety.level.yellow')}: </span>
          {t(`safety.signal.${signal.code}.title`, params)}
        </span>
      </p>
      <p className="mt-1 pl-4 text-muted">{t(`safety.signal.${signal.code}.body`, params)}</p>
      {params.line && (
        <code className="selectable mt-2 ml-4 block overflow-x-auto rounded bg-surface px-2 py-1 font-mono text-xs whitespace-pre">
          {params.file ? `${params.file}: ` : ''}
          {params.line}
        </code>
      )}
    </li>
  )
}

/** Shown before the first install: what will happen, what looks risky, and consent. */
export function SafetyDialog({ gate, onApprove, onCancel }: Props) {
  const { t } = useTranslation()
  const [trust, setTrust] = useState(false)
  const [typed, setTyped] = useState('')
  const report = gate?.report
  const red = report?.signals.filter((s) => s.level === 'red') ?? []
  const yellow = report?.signals.filter((s) => s.level === 'yellow') ?? []
  const confirmed = !report?.needsTypedConfirmation || typed.trim().toLowerCase() === report.confirmWord.toLowerCase()

  const close = () => {
    setTrust(false)
    setTyped('')
    onCancel()
  }
  const approve = () => {
    const trustOwner = trust
    setTrust(false)
    setTyped('')
    onApprove(trustOwner)
  }

  return (
    <Modal
      open={gate != null}
      onClose={close}
      wide
      title={t('safety.title', { name: gate?.name })}
      description={t('safety.intro')}
      footer={
        <>
          <Button onClick={close}>{t('action.cancel')}</Button>
          <Button variant={red.length ? 'danger' : 'primary'} disabled={!confirmed} onClick={approve}>
            {red.length ? t('safety.installAnyway') : t('action.install')}
          </Button>
        </>
      }
    >
      {report && (
        <div className="flex flex-col gap-5">
          {red.length > 0 && (
            <section aria-labelledby="safety-red">
              <h3 id="safety-red" className="mb-2 text-sm font-semibold text-danger">
                {t('safety.redHeading')}
              </h3>
              <ul className="flex flex-col gap-2">
                {red.map((s) => (
                  <Signal key={s.code} signal={s} />
                ))}
              </ul>
            </section>
          )}

          <section aria-labelledby="safety-summary">
            <h3 id="safety-summary" className="mb-2 text-sm font-semibold">
              {t('safety.summaryHeading')}
            </h3>
            <ul className="flex list-disc flex-col gap-1 pl-5 text-sm">
              {report.summary.map((item, i) => (
                <li key={i}>{summaryText(t, item)}</li>
              ))}
            </ul>
            <details className="mt-3 text-sm">
              <summary className="cursor-pointer text-muted hover:text-fg">{t('safety.showCommands')}</summary>
              <pre className="selectable mt-2 overflow-x-auto rounded-lg bg-sunken p-3 font-mono text-xs leading-relaxed">
                {report.commands.join('\n')}
              </pre>
            </details>
          </section>

          {yellow.length > 0 && (
            <section aria-labelledby="safety-yellow">
              <h3 id="safety-yellow" className="mb-2 text-sm font-semibold">
                {t('safety.yellowHeading')}
              </h3>
              <ul className="flex flex-col gap-2">
                {yellow.map((s) => (
                  <Signal key={s.code} signal={s} />
                ))}
              </ul>
            </section>
          )}

          <p className="text-xs text-muted">{t('safety.honest')}</p>

          {report.owner && !report.trustedOwner && (
            <label className="flex items-start gap-2 text-sm">
              <input type="checkbox" checked={trust} onChange={(e) => setTrust(e.target.checked)} className="mt-1" />
              <span>
                {t('safety.trustOwner', { owner: report.owner })}
                <span className="block text-xs text-muted">{t('safety.trustOwnerHint')}</span>
              </span>
            </label>
          )}

          {report.needsTypedConfirmation && (
            <label className="flex flex-col gap-1 text-sm">
              <span>{t('safety.typeToConfirm', { word: report.confirmWord })}</span>
              <input
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                spellCheck={false}
                autoCapitalize="off"
                autoCorrect="off"
                className="min-h-10 rounded-lg border border-border bg-surface px-3 font-mono text-fg focus-visible:border-accent"
              />
            </label>
          )}
        </div>
      )}
    </Modal>
  )
}
