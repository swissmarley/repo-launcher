import { AlertDialog } from 'radix-ui'
import { useTranslation } from 'react-i18next'
import { useReturnFocus } from '../lib/useReturnFocus'
import { Button } from './Button'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
}

/** "Remove all Repo Launcher data": what is deleted, said plainly, before anything happens. */
export function RemoveAllDialog({ open, onOpenChange, onConfirm }: Props) {
  const { t } = useTranslation()
  const returnFocus = useReturnFocus(open)
  return (
    <AlertDialog.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="motion-overlay fixed inset-0 bg-black/40" />
        <AlertDialog.Content onCloseAutoFocus={returnFocus} className="motion-dialog fixed top-1/2 left-1/2 w-[min(30rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-surface p-6 shadow-card">
          <AlertDialog.Title className="text-lg font-semibold">{t('settings.removeAll.title')}</AlertDialog.Title>
          <AlertDialog.Description className="mt-2 text-sm text-muted">{t('settings.removeAll.body')}</AlertDialog.Description>
          <ul className="mt-3 list-disc pl-5 text-sm text-muted">
            <li>{t('settings.removeAll.projects')}</li>
            <li>{t('settings.removeAll.keys')}</li>
            <li>{t('settings.removeAll.shortcuts')}</li>
            <li>{t('settings.removeAll.settings')}</li>
          </ul>
          <p className="mt-3 text-sm text-muted">{t('settings.removeAll.after')}</p>
          <div className="mt-6 flex justify-end gap-2">
            <AlertDialog.Cancel asChild>
              <Button>{t('action.cancel')}</Button>
            </AlertDialog.Cancel>
            <AlertDialog.Action asChild>
              <Button variant="danger" onClick={onConfirm}>
                {t('settings.removeAll.confirm')}
              </Button>
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  )
}
