import { useLayoutEffect, useRef } from 'react'

/**
 * Radix dialogs give focus back to their Trigger when they close; ours are opened from
 * buttons and menus elsewhere, so without this focus would fall to the page itself and
 * keyboard and screen-reader users would lose their place. Pass the result as the
 * content's `onCloseAutoFocus`.
 */
export function useReturnFocus(open: boolean) {
  const opener = useRef<HTMLElement | null>(null)
  // A layout effect runs before the dialog moves focus inside itself.
  useLayoutEffect(() => {
    if (open && document.activeElement instanceof HTMLElement && document.activeElement !== document.body) {
      opener.current = document.activeElement
    }
  }, [open])
  return (event: Event) => {
    const el = opener.current
    opener.current = null
    if (el?.isConnected) {
      event.preventDefault()
      el.focus()
    } else {
      // What opened it is gone (a removed project): start again from the link field.
      const fallback = document.getElementById('add-input')
      if (fallback) {
        event.preventDefault()
        fallback.focus()
      }
    }
  }
}
