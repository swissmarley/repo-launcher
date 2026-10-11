import { useCallback, useEffect, useRef, useState } from 'react'
import type { AppSettings } from '../bindings/AppSettings'
import type { DeviceCode } from '../bindings/DeviceCode'
import type { GitHubAccount } from '../bindings/GitHubAccount'
import type { Problem } from '../bindings/Problem'
import { api, toProblem } from '../lib/api'

export type SignIn =
  | { phase: 'starting' }
  | { phase: 'waiting'; code: DeviceCode }
  | { phase: 'done'; account: GitHubAccount }
  | { phase: 'error'; problem: Problem }

/** App settings and the GitHub connection. */
export function useAccount() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [signIn, setSignIn] = useState<SignIn | null>(null)
  // Resolves the promise returned by `connect` when the dialog closes.
  const done = useRef<((connected: boolean) => void) | null>(null)

  const refresh = useCallback(async () => {
    try {
      setSettings(await api.appSettings())
    } catch {
      // Settings are informational; the app works without them.
    }
  }, [])

  useEffect(() => {
    let active = true
    api
      .appSettings()
      .then((s) => active && setSettings(s))
      .catch(() => {})
    return () => {
      active = false
    }
  }, [])

  const start = useCallback(async () => {
    setSignIn({ phase: 'starting' })
    try {
      const code = await api.githubBeginSignIn()
      setSignIn({ phase: 'waiting', code })
      const account = await api.githubFinishSignIn()
      setSignIn({ phase: 'done', account })
      await refresh()
    } catch (err) {
      const problem = toProblem(err)
      if (problem.code === 'CANCELLED') return
      setSignIn({ phase: 'error', problem })
    }
  }, [refresh])

  /** Opens "Connect GitHub". Resolves true once the user is connected. */
  const connect = useCallback((): Promise<boolean> => {
    void start()
    return new Promise((resolve) => {
      done.current = resolve
    })
  }, [start])

  const closeSignIn = useCallback(() => {
    void api.githubCancelSignIn()
    setSignIn((s) => {
      done.current?.(s?.phase === 'done')
      done.current = null
      return null
    })
  }, [])

  const signOut = useCallback(async () => {
    const revokeUrl = await api.githubSignOut()
    await refresh()
    return revokeUrl
  }, [refresh])

  return {
    settings,
    account: settings?.githubAccount ?? null,
    signIn,
    connect,
    retrySignIn: start,
    closeSignIn,
    signOut,
    refresh,
  }
}

export type Account = ReturnType<typeof useAccount>
