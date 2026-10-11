import { useCallback, useEffect, useRef, useState } from 'react'
import type { Problem } from '../bindings/Problem'
import { api, type ShellSettings } from '../lib/api'

export interface ShellHandlers {
  onLaunchProblem: (projectId: string, problem: Problem) => void
  onLaunchInUi: (projectId: string) => void
  onQuitRequested: () => void
  onOrphans: (projectIds: string[]) => void
  onUpdateAvailable: (version: string) => void
}

/** The app shell around the window: starting at login, and what shortcuts and the tray ask for. */
export function useShell(handlers: ShellHandlers) {
  const [settings, setSettings] = useState<ShellSettings | null>(null)
  const latest = useRef(handlers)
  useEffect(() => {
    latest.current = handlers
  })

  const refresh = useCallback(async () => {
    try {
      setSettings(await api.shellSettings())
    } catch {
      // Informational; the app works without it.
    }
  }, [])

  useEffect(() => {
    let active = true
    api
      .shellSettings()
      .then((s) => active && setSettings(s))
      .catch(() => {})
    // Messages wait in the shell until taken, so none is lost while the window loads.
    const drain = async () => {
      for (const m of await api.takeMessages()) {
        const h = latest.current
        if (m.type === 'launchProblem') h.onLaunchProblem(m.projectId, m.problem)
        else if (m.type === 'launchInUi') h.onLaunchInUi(m.projectId)
        else if (m.type === 'quitRequested') h.onQuitRequested()
        else if (m.type === 'orphans') h.onOrphans(m.projectIds)
        else if (m.type === 'updateAvailable') h.onUpdateAvailable(m.version)
      }
    }
    void drain()
    const off = api.onShell(() => void drain())
    return () => {
      active = false
      void off.then((f) => f())
    }
  }, [])

  const setLaunchAtLogin = useCallback(
    async (enabled: boolean) => {
      await api.setLaunchAtLogin(enabled)
      await refresh()
    },
    [refresh],
  )

  return { settings, refresh, setLaunchAtLogin }
}

export type Shell = ReturnType<typeof useShell>
