import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '../lib/api'
import type { SignIn } from '../state/useAccount'
import { Button } from './Button'
import { Modal } from './Modal'
import { ProblemNotice } from './ProblemNotice'

interface Props {
  signIn: SignIn | null
  onRetry: () => void
  onClose: () => void
}

/** "Connect GitHub": a short code the user enters on github.com, then we wait. */
export function SignInDialog({ signIn, onRetry, onClose }: Props) {
  const { t } = useTranslation()
  const [copied, setCopied] = useState(false)

  const copyAndOpen = async (code: string, url: string) => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
    } catch {
      // The code is on screen; copying is a convenience.
    }
    await api.openExternal(url).catch(() => {})
  }

  let body = null
  let footer = <Button onClick={onClose}>{t('action.cancel')}</Button>
  switch (signIn?.phase) {
    case 'starting':
      body = <p className="text-sm text-muted">{t('signIn.starting')}</p>
      break
    case 'waiting':
      body = (
        <div className="flex flex-col items-center gap-4 text-center">
          <p className="text-sm text-muted">{t('signIn.instructions')}</p>
          <p
            aria-label={t('signIn.codeLabel')}
            className="selectable rounded-lg bg-sunken px-6 py-3 font-mono text-3xl font-semibold tracking-[0.2em]"
          >
            {signIn.code.userCode}
          </p>
          <Button variant="primary" size="lg" onClick={() => copyAndOpen(signIn.code.userCode, signIn.code.verificationUri)}>
            {copied ? t('signIn.copiedOpen') : t('signIn.copyOpen')}
          </Button>
          <p className="flex items-center gap-2 text-sm text-muted" aria-live="polite">
            <span aria-hidden className="size-2 animate-pulse rounded-full bg-accent" />
            {t('signIn.waiting')}
          </p>
          <p className="text-xs text-muted">{t('signIn.permissions')}</p>
        </div>
      )
      break
    case 'done':
      body = <p className="text-sm">{t('signIn.connected', { login: signIn.account.login })}</p>
      footer = (
        <Button variant="primary" onClick={onClose}>
          {t('action.done')}
        </Button>
      )
      break
    case 'error':
      body = <ProblemNotice problem={signIn.problem} onFix={onRetry} />
      break
  }

  return (
    <Modal open={signIn != null} onClose={onClose} title={t('signIn.title')} footer={footer}>
      {body}
    </Modal>
  )
}
