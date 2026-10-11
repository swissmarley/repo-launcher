import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'

interface Props {
  onAdd: (input: string) => Promise<Problem | null>
  /** Handles a problem's fix; resolves true when adding should be tried again (signed in). */
  onFix: (p: Problem) => Promise<boolean>
  autoFocus?: boolean
}

export function AddBar({ onAdd, onFix, autoFocus }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState('')
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<Problem | null>(null)
  const input = useRef<HTMLInputElement>(null)

  // `autoFocus` only works on mount; the library is known to be empty a moment later.
  useEffect(() => {
    if (autoFocus) input.current?.focus()
  }, [autoFocus])

  const submit = async (e?: FormEvent) => {
    e?.preventDefault()
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
          ref={input}
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
          <ProblemNotice
            problem={problem}
            onFix={async () => {
              if (await onFix(problem)) void submit()
            }}
            onDismiss={() => setProblem(null)}
          />
        </div>
      )}
    </div>
  )
}
