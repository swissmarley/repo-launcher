import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { EnvVarView } from '../bindings/EnvVarView'
import type { Problem } from '../bindings/Problem'
import { api } from '../lib/api'
import type { KeysForm } from '../state/useProjects'
import { Button } from './Button'
import { Modal } from './Modal'
import { ProblemNotice } from './ProblemNotice'

interface Props {
  form: KeysForm | null
  onSave: (values: Record<string, string | null>, launchAfter: boolean) => Promise<Problem | null>
  onClose: () => void
}

function labelOf(t: ReturnType<typeof useTranslation>['t'], v: EnvVarView) {
  if (v.label) return v.label
  if (v.known) return t(`keys.kind.${v.known.kind}`, { service: v.known.service })
  return v.name
}

/** The settings a project asks for (API keys etc.). Secret values go to the system keychain.
 *  Render with `key={projectId}` so each project starts with a clean form. */
export function KeysDialog({ form, onSave, onClose }: Props) {
  const { t } = useTranslation()
  // Only fields the user touched are saved; an emptied field clears its value.
  const [edits, setEdits] = useState<Record<string, string>>({})
  const [problem, setProblem] = useState<Problem | null>(null)
  const [saving, setSaving] = useState(false)

  const vars = form?.vars ?? []
  const filled = (v: EnvVarView) => (v.name in edits ? edits[v.name].trim() !== '' : v.hasValue)
  const missingRequired = vars.some((v) => v.required && !filled(v))

  const save = async (launchAfter: boolean) => {
    setSaving(true)
    const values = Object.fromEntries(Object.entries(edits).map(([k, v]) => [k, v.trim() === '' ? null : v]))
    const p = await onSave(values, launchAfter)
    setSaving(false)
    setProblem(p)
  }

  return (
    <Modal
      open={form != null}
      onClose={onClose}
      wide
      title={form?.thenLaunch ? t('keys.titleBeforeLaunch', { name: form?.name }) : t('keys.title', { name: form?.name })}
      description={vars.length ? t('keys.intro') : t('keys.none')}
      footer={
        form?.thenLaunch ? (
          <>
            <Button onClick={onClose}>{t('keys.notNow')}</Button>
            {!missingRequired && vars.some((v) => !v.required) && Object.keys(edits).length === 0 && (
              <Button onClick={() => save(true)} disabled={saving}>
                {t('keys.launchWithout')}
              </Button>
            )}
            <Button variant="primary" onClick={() => save(true)} disabled={saving || missingRequired}>
              {t('keys.saveAndLaunch')}
            </Button>
          </>
        ) : (
          <>
            <Button onClick={onClose}>{t('action.cancel')}</Button>
            <Button variant="primary" onClick={() => save(false)} disabled={saving || vars.length === 0}>
              {t('keys.save')}
            </Button>
          </>
        )
      }
    >
      {vars.length > 0 && (
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (!missingRequired) void save(!!form?.thenLaunch)
          }}
        >
          {vars.map((v) => {
            const id = `key-${v.name}`
            const value = v.name in edits ? edits[v.name] : (v.value ?? '')
            return (
              <div key={v.name} className="flex flex-col gap-1">
                <label htmlFor={id} className="flex items-baseline justify-between gap-2 text-sm font-medium">
                  <span>
                    {/* A label from the project itself, or the variable's name, isn't ours to translate. */}
                    <span translate={v.label || !v.known ? 'no' : undefined}>{labelOf(t, v)}</span>
                    {v.required && <span className="ml-2 text-xs font-normal text-muted">{t('keys.required')}</span>}
                  </span>
                  <span translate="no" className="font-mono text-xs font-normal text-muted">
                    {v.name}
                  </span>
                </label>
                {v.description && (
                  <p translate="no" className="text-xs text-muted">
                    {v.description}
                  </p>
                )}
                <input
                  id={id}
                  type={v.secret ? 'password' : 'text'}
                  autoComplete="off"
                  spellCheck={false}
                  value={value}
                  placeholder={v.secret && v.hasValue ? t('keys.savedPlaceholder') : ''}
                  onChange={(e) => setEdits((prev) => ({ ...prev, [v.name]: e.target.value }))}
                  className="min-h-10 rounded-lg border border-border bg-surface px-3 font-mono text-sm text-fg placeholder:font-sans placeholder:text-muted focus-visible:border-accent"
                />
                <div className="flex gap-3 text-xs">
                  {v.helpUrl && (
                    <button type="button" className="text-accent hover:underline" onClick={() => api.openExternal(v.helpUrl!)}>
                      {t('keys.whereToGet')}
                    </button>
                  )}
                  {v.secret && v.hasValue && !(v.name in edits) && (
                    <button type="button" className="text-muted hover:text-fg" onClick={() => setEdits((prev) => ({ ...prev, [v.name]: '' }))}>
                      {t('keys.clear')}
                    </button>
                  )}
                </div>
              </div>
            )
          })}
          <p className="text-xs text-muted">{t('keys.storage')}</p>
          {problem && <ProblemNotice problem={problem} onDismiss={() => setProblem(null)} />}
          <button type="submit" hidden />
        </form>
      )}
    </Modal>
  )
}
