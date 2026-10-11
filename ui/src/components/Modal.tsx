import { Dialog } from 'radix-ui'
import type { ReactNode } from 'react'
import { useReturnFocus } from '../lib/useReturnFocus'

interface Props {
  open: boolean
  onClose: () => void
  title: ReactNode
  description?: ReactNode
  children?: ReactNode
  footer?: ReactNode
  /** Wider dialogs for content with lists (safety summary, settings). */
  wide?: boolean
}

/** A centered dialog: title, scrollable body, actions at the bottom. */
export function Modal({ open, onClose, title, description, children, footer, wide }: Props) {
  const returnFocus = useReturnFocus(open)
  return (
    <Dialog.Root open={open} onOpenChange={(o) => !o && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="motion-overlay fixed inset-0 bg-black/40" />
        <Dialog.Content
          onCloseAutoFocus={returnFocus}
          className={`motion-dialog fixed top-1/2 left-1/2 flex max-h-[calc(100vh-3rem)] -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl border border-border bg-surface shadow-card ${
            wide ? 'w-[min(40rem,calc(100vw-2rem))]' : 'w-[min(30rem,calc(100vw-2rem))]'
          }`}
        >
          <div className="px-6 pt-6">
            <Dialog.Title className="text-lg font-semibold">{title}</Dialog.Title>
            {description ? (
              <Dialog.Description className="mt-1 text-sm text-muted">{description}</Dialog.Description>
            ) : (
              <Dialog.Description className="sr-only">{title}</Dialog.Description>
            )}
          </div>
          {children && <div className="mt-4 flex-1 overflow-auto px-6">{children}</div>}
          {footer && <div className="mt-6 flex flex-wrap items-center justify-end gap-2 px-6 pb-6">{footer}</div>}
          {!footer && <div className="pb-6" />}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
