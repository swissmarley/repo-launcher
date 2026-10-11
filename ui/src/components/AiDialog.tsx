import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { AiOutcome } from '../bindings/AiOutcome'
import type { Preview } from '../bindings/Preview'
import type { Problem } from '../bindings/Problem'
import { api, toProblem } from '../lib/api'
import { Button } from './Button'
import { Modal } from './Modal'
import { ProblemNotice } from './ProblemNotice'

export interface AiTarget {
  projectId: string
  name: string
}

interface Props {
  target: AiTarget | null
  onClose: () => void
  /** A plan was saved: continue with the safety review and install. */
  onReview: (projectId: string) => void
  onFix: (p: Problem) => void
}

type Phase =
  | { kind: 'loading' }
  | { kind: 'preview'; preview: Preview }
  | { kind: 'sending'; preview: Preview }
  | { kind: 'done'; outcome: AiOutcome }
  | { kind: 'error'; problem: Problem }

/** Asking AI how to start a project: shows exactly what will be sent before sending it.
 *  Render with `key={projectId}` so each request starts fresh. */
export function AiDialog({ target, onClose, onReview, onFix }: Props) {
  const { t } = useTranslation()
  const [phase, setPhase] = useState<Phase>({ kind: 'loading' })

  useEffect(() => {
    if (!target) return
    let active = true
    api
      .aiPreview(target.projectId)
      .then((preview) => active && setPhase({ kind: 'preview', preview }))
      .catch((err) => active && setPhase({ kind: 'error', problem: toProblem(err) }))
    return () => {
      active = false
    }
  }, [target])

  const send = async (preview: Preview) => {
    if (!target) return
    setPhase({ kind: 'sending', preview })
    try {
      setPhase({ kind: 'done', outcome: await api.aiSuggest(target.projectId, preview.digest) })
    } catch (err) {
      setPhase({ kind: 'error', problem: toProblem(err) })
    }
  }

  let body = null
  let footer = <Button onClick={onClose}>{t('action.cancel')}</Button>
  switch (phase.kind) {
    case 'loading':
      body = <p className="text-sm text-muted">{t('ai.preparing')}</p>
      break
    case 'preview':
    case 'sending': {
      const p = phase.preview
      body = (
        <div className="flex flex-col gap-3 text-sm">
          <p>{t('ai.willSend', { model: p.model })}</p>
          <ul className="list-disc pl-5 text-muted">
            <li>{t('ai.fileNames', { count: p.listedFiles })}</li>
            {p.includedFiles.length > 0 && <li>{t('ai.fileContents', { files: p.includedFiles.join(', ') })}</li>}
            <li>{t('ai.instructions')}</li>
          </ul>
          <p className="text-muted">{t('ai.neverSent')}</p>
          <details>
            <summary className="cursor-pointer text-muted hover:text-fg">{t('ai.showExact')}</summary>
            <pre className="selectable mt-2 max-h-72 overflow-auto rounded-lg bg-sunken p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap">
              {`${p.system}\n\n────────\n\n${p.message}`}
            </pre>
          </details>
          <p className="text-xs text-muted">{t('ai.afterwards')}</p>
        </div>
      )
      footer = (
        <>
          <Button onClick={onClose}>{t('action.cancel')}</Button>
          <Button variant="primary" disabled={phase.kind === 'sending'} onClick={() => send(p)}>
            {phase.kind === 'sending' ? t('ai.sending') : t('ai.send')}
          </Button>
        </>
      )
      break
    }
    case 'done':
      body = (
        <div className="flex flex-col gap-2 text-sm">
          <p className="font-medium">{phase.outcome.recipeSaved ? t('ai.planReady') : t('ai.notRunnable')}</p>
          {phase.outcome.explanation && <p className="text-muted">“{phase.outcome.explanation}”</p>}
        </div>
      )
      footer = phase.outcome.recipeSaved ? (
        <>
          <Button onClick={onClose}>{t('action.close')}</Button>
          <Button variant="primary" onClick={() => target && onReview(target.projectId)}>
            {t('ai.review')}
          </Button>
        </>
      ) : (
        <Button onClick={onClose}>{t('action.close')}</Button>
      )
      break
    case 'error':
      body = <ProblemNotice problem={phase.problem} onFix={() => onFix(phase.problem)} />
      footer = <Button onClick={onClose}>{t('action.close')}</Button>
      break
  }

  return (
    <Modal open={target != null} onClose={onClose} wide title={t('ai.title', { name: target?.name })} footer={footer}>
      {body}
    </Modal>
  )
}
