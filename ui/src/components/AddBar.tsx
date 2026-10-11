import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'

interface Props {
  onAdd: (input: string) => Promise<Problem | null>
  autoFocus?: boolean
}

export function AddBar({ onAdd, autoFocus }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState('')
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<Problem | null>(null)

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    if (!value.trim() || busy) return
    setBusy(true)
    setProblem(null)
    const p = await onAdd(value)
    setBusy(false)
    if (p) setProblem(p)
    else setValue('')
  }

  return (
    <div className="w-full">
      <form onSubmit={submit} className="flex gap-2">
        <label htmlFor="add-input" className="sr-only">
          {t('add.label')}
        </label>
        <input
          id="add-input"
          autoFocus={autoFocus}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={t('add.placeholder')}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          className="min-h-11 flex-1 rounded-lg border border-border bg-surface px-4 text-base text-fg placeholder:text-muted focus-visible:border-accent"
        />
        <Button type="submit" variant="primary" size="lg" disabled={busy || !value.trim()}>
          {busy ? t('add.adding') : t('add.button')}
        </Button>
      </form>
      {problem && (
        <div className="mt-3">
          <ProblemNotice problem={problem} onDismiss={() => setProblem(null)} />
        </div>
      )}
    </div>
  )
}
