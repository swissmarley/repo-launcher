import { AlertDialog } from 'radix-ui'
import { useTranslation } from 'react-i18next'
import { useReturnFocus } from '../lib/useReturnFocus'
import { Button } from './Button'

interface Props {
  name: string
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
}

export function RemoveDialog({ name, open, onOpenChange, onConfirm }: Props) {
  const { t } = useTranslation()
  const returnFocus = useReturnFocus(open)
  return (
    <AlertDialog.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="motion-overlay fixed inset-0 bg-black/40" />
        <AlertDialog.Content onCloseAutoFocus={returnFocus} className="motion-dialog fixed top-1/2 left-1/2 w-[min(28rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-surface p-6 shadow-card">
          <AlertDialog.Title className="text-lg font-semibold">{t('remove.title', { name })}</AlertDialog.Title>
          <AlertDialog.Description className="mt-2 text-sm text-muted">{t('remove.body')}</AlertDialog.Description>
          <div className="mt-6 flex justify-end gap-2">
            <AlertDialog.Cancel asChild>
              <Button>{t('action.cancel')}</Button>
            </AlertDialog.Cancel>
            <AlertDialog.Action asChild>
              <Button variant="danger" onClick={onConfirm}>
                {t('remove.confirm')}
              </Button>
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  )
}
