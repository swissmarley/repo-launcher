import { Dialog } from 'radix-ui'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '../lib/api'
import { useReturnFocus } from '../lib/useReturnFocus'
import { Button } from './Button'

const MAX_LINES = 2000

interface Props {
  projectId: string
  name: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Live output from installing and running a project. Hidden by default ("Show activity"). */
export function ActivitySheet({ projectId, name, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const returnFocus = useReturnFocus(open)
  const [lines, setLines] = useState<string[]>([])
  const bottom = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    let active = true
    void api.projectLog(projectId).then((l) => active && setLines(l))
    const off = api.onEvent((e) => {
      if (e.type === 'log' && e.projectId === projectId) {
        setLines((prev) => [...prev.slice(-(MAX_LINES - 1)), e.line])
      }
    })
    return () => {
      active = false
      void off.then((f) => f())
    }
  }, [open, projectId])

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'end' })
  }, [lines])

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="motion-overlay fixed inset-0 bg-black/40" />
        <Dialog.Content onCloseAutoFocus={returnFocus} className="motion-sheet fixed inset-x-6 top-10 bottom-10 mx-auto flex max-w-4xl flex-col rounded-xl border border-border bg-surface shadow-card">
          <div className="flex items-center justify-between border-b border-border px-5 py-3">
            <Dialog.Title className="text-base font-semibold">{t('activity.title', { name })}</Dialog.Title>
            <Dialog.Close asChild>
              <Button variant="ghost">{t('action.close')}</Button>
            </Dialog.Close>
          </div>
          <Dialog.Description className="sr-only">{t('activity.title', { name })}</Dialog.Description>
          <div className="selectable flex-1 overflow-auto bg-sunken px-5 py-3 font-mono text-xs leading-relaxed text-fg">
            {lines.length === 0 ? (
              <p className="text-muted">{t('activity.empty')}</p>
            ) : (
              lines.map((line, i) => (
                <div key={i} translate="no" className="whitespace-pre-wrap break-all">
                  {line}
                </div>
              ))
            )}
            <div ref={bottom} />
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
