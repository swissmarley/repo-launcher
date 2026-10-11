import { useTranslation } from 'react-i18next'
import type { Updates } from '../state/useUpdates'
import { Button } from './Button'
import { ProblemNotice } from './ProblemNotice'
import { ProgressBar } from './ProgressBar'

interface Props {
  updates: Updates
  /** Starts the install, asking first if projects are running. */
  onInstall: () => void
}

/** "A new version is ready": shown at the top of the library until installed or put off. */
export function UpdateBanner({ updates: u, onInstall }: Props) {
  const { t } = useTranslation()
  if (u.installProblem && u.progress == null) {
    return (
      <div className="mb-4">
        <ProblemNotice problem={u.installProblem} onFix={onInstall} onDismiss={u.dismissInstallProblem} />
      </div>
    )
  }
  if (!u.available || (u.dismissed && u.progress == null)) return null
  if (u.progress != null) {
    const label = t('updates.downloading', { version: u.available })
    return (
      <div role="status" className="mb-4 flex flex-col gap-2 rounded-lg bg-accent-soft px-4 py-3 text-sm">
        <p>{label}</p>
        <ProgressBar value={u.progress} label={label} />
      </div>
    )
  }
  return (
    <div role="status" className="motion-enter mb-4 flex flex-wrap items-center gap-3 rounded-lg bg-accent-soft px-4 py-3 text-sm">
      <p className="flex-1">{t('updates.ready', { version: u.available })}</p>
      <Button variant="primary" onClick={onInstall}>
        {t('updates.install')}
      </Button>
      <Button variant="ghost" onClick={u.dismiss}>
        {t('updates.later')}
      </Button>
    </div>
  )
}
