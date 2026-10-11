import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FixAction } from '../bindings/FixAction'
import type { Problem } from '../bindings/Problem'
import { Button } from './Button'

interface Props {
  problem: Problem
  onFix?: (fix: FixAction) => void
  onDismiss?: () => void
}

/** One human sentence, a primary fix, and diagnostics for whoever helps the user. */
export function ProblemNotice({ problem, onFix, onDismiss }: Props) {
  const { t } = useTranslation()
  const [copied, setCopied] = useState(false)
  const params = problem.params as Record<string, string>
  const key = `problem.${problem.code}`
  const body = problem.code === 'EXITED_EARLY' && params.module ? t(`${key}.bodyModule`, params) : t(`${key}.body`, params)
  const fixKind = problem.fix?.kind
  const fixLabel = fixKind && onFix && t(`fix.${fixKind}`, { defaultValue: '' })

  const copy = async () => {
    const text = [`code: ${problem.code}`, `params: ${JSON.stringify(problem.params)}`, problem.detail ?? '']
      .join('\n')
      .trim()
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Clipboard can be unavailable; nothing useful to tell the user here.
    }
  }

  return (
    <div role="alert" className="rounded-lg border border-danger/30 bg-danger-soft p-3 text-sm">
      <p className="font-semibold text-fg">{t(`${key}.title`, params)}</p>
      {body && <p className="mt-1 text-muted">{body}</p>}
      <div className="mt-3 flex flex-wrap gap-2">
        {fixLabel && (
          <Button variant="primary" onClick={() => problem.fix && onFix?.(problem.fix)}>
            {fixLabel}
          </Button>
        )}
        {problem.detail && (
          <Button variant="ghost" onClick={copy}>
            {copied ? t('problem.copied') : t('problem.copyDiagnostics')}
          </Button>
        )}
        {onDismiss && (
          <Button variant="ghost" onClick={onDismiss} className="ml-auto">
            {t('problem.dismiss')}
          </Button>
        )}
      </div>
    </div>
  )
}
