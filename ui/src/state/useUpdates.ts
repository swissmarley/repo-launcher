import { useCallback, useEffect, useState } from 'react'
import type { Problem } from '../bindings/Problem'
import { api, toProblem, type UpdatesInfo } from '../lib/api'

/** Updating Repo Launcher itself: the daily check's news, checking by hand, installing. */
export function useUpdates() {
  const [info, setInfo] = useState<UpdatesInfo | null>(null)
  /** A newer version, found by the daily check or by hand. */
  const [available, setAvailable] = useState<string | null>(null)
  /** The last check by hand found nothing newer. */
  const [upToDate, setUpToDate] = useState(false)
  const [checking, setChecking] = useState(false)
  /** Download progress while installing (0 to 1), or null when not installing. */
  const [progress, setProgress] = useState<number | null>(null)
  /** Checking by hand failed (shown in Settings). */
  const [problem, setProblem] = useState<Problem | null>(null)
  /** Downloading or installing failed (shown on the banner, with "Try again"). */
  const [installProblem, setInstallProblem] = useState<Problem | null>(null)
  const [dismissed, setDismissed] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const i = await api.updatesInfo()
      setInfo(i)
      if (i.available) setAvailable(i.available)
    } catch {
      // Informational; the app works without it.
    }
  }, [])

  useEffect(() => {
    let active = true
    api
      .updatesInfo()
      .then((i) => {
        if (!active) return
        setInfo(i)
        if (i.available) setAvailable(i.available)
      })
      .catch(() => {})
    return () => {
      active = false
    }
  }, [])

  const check = useCallback(async () => {
    setChecking(true)
    setProblem(null)
    setUpToDate(false)
    try {
      const version = await api.checkForUpdate()
      setAvailable(version)
      setUpToDate(version == null)
      if (version) setDismissed(false)
    } catch (err) {
      setProblem(toProblem(err))
    } finally {
      setChecking(false)
    }
  }, [])

  const install = useCallback(async () => {
    setInstallProblem(null)
    setProgress(0)
    const off = await api.onUpdateProgress(setProgress)
    try {
      // On success the app restarts into the new version.
      await api.installUpdate()
    } catch (err) {
      setInstallProblem(toProblem(err))
      setProgress(null)
      void refresh()
    } finally {
      off()
    }
  }, [refresh])

  const setAutoCheck = useCallback(
    async (enabled: boolean) => {
      await api.setAutoUpdateCheck(enabled)
      await refresh()
    },
    [refresh],
  )

  return {
    info,
    available,
    upToDate,
    checking,
    progress,
    problem,
    installProblem,
    /** The banner was closed with "Later". */
    dismissed,
    dismiss: () => setDismissed(true),
    dismissProblem: () => setProblem(null),
    dismissInstallProblem: () => setInstallProblem(null),
    /** The daily check found something (a shell message). */
    found: (version: string) => {
      setAvailable(version)
      setDismissed(false)
    },
    check,
    install,
    setAutoCheck,
  }
}

export type Updates = ReturnType<typeof useUpdates>
