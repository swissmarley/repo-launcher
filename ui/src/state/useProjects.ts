import { useCallback, useEffect, useReducer, useRef, useState } from 'react'
import type { EnvVarView } from '../bindings/EnvVarView'
import type { Problem } from '../bindings/Problem'
import type { SafetyReport } from '../bindings/SafetyReport'
import type { ProjectStatus } from '../bindings/ProjectStatus'
import type { RunState } from '../bindings/RunState'
import type { Stage } from '../bindings/Stage'
import type { StepLabel } from '../bindings/StepLabel'
import { api, toProblem, type ProjectView } from '../lib/api'

export type Busy = 'review' | 'install' | 'launch' | 'stop' | 'remove' | 'update'
export type Action = 'install' | 'launch' | 'update' | 'repair'

export interface Live {
  progress?: { stage: Stage; fraction: number; label: StepLabel | null }
  runState?: RunState
  problem?: Problem | null
  busy?: Busy
  /** What to repeat when the user clicks "Try again". */
  lastAction?: Action
  /** A short, non-error message such as "You have the latest version.": an i18n key. */
  notice?: { key: string; params?: Record<string, string> } | null
}

/** The safety gate waiting for the user's decision. */
export interface Gate {
  projectId: string
  name: string
  report: SafetyReport
}

/** The settings form; `thenLaunch` launches after saving. */
export interface KeysForm {
  projectId: string
  name: string
  vars: EnvVarView[]
  thenLaunch: boolean
}

interface State {
  projects: ProjectView[]
  live: Record<string, Live>
  loaded: boolean
}

type Msg =
  | { type: 'loaded'; projects: ProjectView[] }
  | { type: 'upsert'; project: ProjectView }
  | { type: 'remove'; id: string }
  | { type: 'live'; id: string; patch: Partial<Live> }
  | { type: 'running'; id: string; running: boolean; url: string | null }
  | { type: 'status'; id: string; status: ProjectStatus }

function reducer(state: State, msg: Msg): State {
  switch (msg.type) {
    case 'loaded':
      return { ...state, projects: msg.projects, loaded: true }
    case 'upsert': {
      const exists = state.projects.some((p) => p.id === msg.project.id)
      const projects = exists
        ? state.projects.map((p) => (p.id === msg.project.id ? msg.project : p))
        : [msg.project, ...state.projects]
      return { ...state, projects }
    }
    case 'remove': {
      const live = { ...state.live }
      delete live[msg.id]
      return { ...state, projects: state.projects.filter((p) => p.id !== msg.id), live }
    }
    case 'live':
      return { ...state, live: { ...state.live, [msg.id]: { ...state.live[msg.id], ...msg.patch } } }
    case 'status':
      return { ...state, projects: state.projects.map((p) => (p.id === msg.id ? { ...p, status: msg.status } : p)) }
    case 'running':
      return {
        ...state,
        projects: state.projects.map((p) => (p.id === msg.id ? { ...p, running: msg.running, url: msg.url } : p)),
      }
  }
}

export function useProjects() {
  const [state, dispatch] = useReducer(reducer, { projects: [], live: {}, loaded: false })
  const patch = useCallback((id: string, p: Partial<Live>) => dispatch({ type: 'live', id, patch: p }), [])

  const refresh = useCallback(async () => {
    dispatch({ type: 'loaded', projects: await api.listProjects() })
  }, [])

  useEffect(() => {
    void refresh()
    const unlisten = api.onEvent((e) => {
      if (e.type === 'progress') {
        patch(e.projectId, { progress: { stage: e.stage, fraction: e.fraction, label: e.label } })
      } else if (e.type === 'runState') {
        patch(e.projectId, { runState: e.state })
        const running = e.state === 'starting' || e.state === 'running'
        dispatch({ type: 'running', id: e.projectId, running, url: e.url })
        if (e.state === 'crashed') {
          patch(e.projectId, {
            problem: { code: 'EXITED_EARLY', params: {}, fix: { kind: 'retry' }, detail: `exit code ${e.exitCode ?? 'unknown'}` },
            lastAction: 'launch',
          })
        }
      }
    })
    return () => {
      void unlisten.then((off) => off())
    }
  }, [refresh, patch])

  const add = useCallback(async (input: string): Promise<Problem | null> => {
    try {
      dispatch({ type: 'upsert', project: await api.addProject(input) })
      return null
    } catch (err) {
      return toProblem(err)
    }
  }, [])

  const [gate, setGate] = useState<Gate | null>(null)
  const [keys, setKeys] = useState<KeysForm | null>(null)
  const projectsRef = useRef(state.projects)
  useEffect(() => {
    projectsRef.current = state.projects
  }, [state.projects])
  const nameOf = (id: string) => projectsRef.current.find((p) => p.id === id)?.name ?? ''

  const runInstall = useCallback(
    async (id: string) => {
      patch(id, { busy: 'install', problem: null, progress: undefined, lastAction: 'install' })
      dispatch({ type: 'status', id, status: 'installing' })
      try {
        dispatch({ type: 'upsert', project: await api.installProject(id) })
      } catch (err) {
        const problem = toProblem(err)
        patch(id, { problem: problem.code === 'CANCELLED' ? null : problem })
        await refresh()
      } finally {
        patch(id, { busy: undefined, progress: undefined })
      }
    },
    [patch, refresh],
  )

  /** Install starts with the safety gate: download, show what will happen, then ask. */
  const install = useCallback(
    async (id: string) => {
      patch(id, { busy: 'review', problem: null, progress: undefined, lastAction: 'install', notice: null })
      let report: SafetyReport
      try {
        report = await api.reviewProject(id)
      } catch (err) {
        const problem = toProblem(err)
        patch(id, { busy: undefined, progress: undefined, problem: problem.code === 'CANCELLED' ? null : problem })
        return
      }
      patch(id, { busy: undefined, progress: undefined })
      if (report.consented) return runInstall(id)
      setGate({ projectId: id, name: nameOf(id), report })
    },
    [patch, runInstall],
  )

  const approveGate = useCallback(
    async (trustOwner: boolean) => {
      if (!gate) return
      const { projectId, report } = gate
      setGate(null)
      try {
        await api.consentProject(projectId, report.fingerprint, trustOwner)
      } catch (err) {
        patch(projectId, { problem: toProblem(err) })
        return
      }
      await runInstall(projectId)
    },
    [gate, patch, runInstall],
  )

  const runLaunch = useCallback(
    async (id: string) => {
      patch(id, { busy: 'launch', problem: null, lastAction: 'launch' })
      try {
        await api.launchProject(id)
      } catch (err) {
        patch(id, { problem: toProblem(err) })
      } finally {
        patch(id, { busy: undefined })
      }
    },
    [patch],
  )

  const openKeys = useCallback(
    async (id: string, thenLaunch: boolean) => {
      try {
        const vars = await api.envVars(id)
        setKeys({ projectId: id, name: nameOf(id), vars, thenLaunch })
      } catch (err) {
        patch(id, { problem: toProblem(err) })
      }
    },
    [patch],
  )

  /** The first launch shows the settings form once if the project asks for keys. */
  const launch = useCallback(
    async (id: string) => {
      const p = projectsRef.current.find((x) => x.id === id)
      if (p && !p.keysReviewed) {
        const vars = await api.envVars(id).catch(() => [])
        if (vars.some((v) => !v.hasValue)) {
          setKeys({ projectId: id, name: p.name, vars, thenLaunch: true })
          return
        }
      }
      await runLaunch(id)
    },
    [runLaunch],
  )

  /** Saves the form's values; `null` entries clear a value, missing ones are left alone. */
  const saveKeys = useCallback(
    async (values: Record<string, string | null>, launchAfter: boolean): Promise<Problem | null> => {
      if (!keys) return null
      const id = keys.projectId
      try {
        for (const [name, value] of Object.entries(values)) {
          await api.setEnvValue(id, name, value)
        }
        await api.markKeysReviewed(id)
      } catch (err) {
        return toProblem(err)
      }
      setKeys(null)
      dispatch({ type: 'upsert', project: await api.getProject(id) })
      if (launchAfter) await runLaunch(id)
      return null
    },
    [keys, runLaunch],
  )

  const stop = useCallback(
    async (id: string) => {
      patch(id, { busy: 'stop' })
      await api.stopProject(id).catch(() => {})
      patch(id, { busy: undefined })
    },
    [patch],
  )

  const remove = useCallback(async (id: string) => {
    patch(id, { busy: 'remove' })
    try {
      await api.uninstallProject(id)
      dispatch({ type: 'remove', id })
    } catch (err) {
      patch(id, { busy: undefined, problem: toProblem(err) })
    }
  }, [patch])

  /** Update and Repair both reinstall; they share the install progress UI. */
  const reinstall = useCallback(
    async (id: string, action: 'update' | 'repair') => {
      patch(id, { busy: action === 'update' ? 'update' : 'install', problem: null, notice: null, progress: undefined, lastAction: action })
      try {
        if (action === 'update') {
          const updated = await api.updateProject(id)
          patch(id, { notice: { key: updated ? 'update.updated' : 'update.upToDate' } })
        } else {
          dispatch({ type: 'status', id, status: 'installing' })
          await api.repairProject(id)
        }
      } catch (err) {
        const problem = toProblem(err)
        // The update changes what gets installed or run: ask again before going on.
        if (problem.code === 'CONSENT_REQUIRED') {
          patch(id, { busy: undefined, progress: undefined })
          await refresh()
          return install(id)
        }
        patch(id, { problem })
      } finally {
        patch(id, { busy: undefined, progress: undefined })
        await refresh()
      }
    },
    [patch, refresh, install],
  )

  /** Re-reads one project (its shortcuts and settings changed). */
  const reload = useCallback(async (id: string) => {
    dispatch({ type: 'upsert', project: await api.getProject(id) })
  }, [])

  /** Adds or removes the project's shortcut; `platform` picks the confirmation's wording. */
  const toggleShortcut = useCallback(
    async (id: string, platform: string) => {
      const p = projectsRef.current.find((x) => x.id === id)
      if (!p) return
      try {
        if (p.shortcuts.length) {
          await api.removeShortcut(id)
          patch(id, { notice: { key: 'shortcut.removed' }, problem: null })
        } else {
          await api.createShortcut(id)
          const key = ['macos', 'windows', 'linux'].includes(platform) ? platform : 'linux'
          patch(id, { notice: { key: `shortcut.created.${key}`, params: { name: p.name } }, problem: null })
        }
      } catch (err) {
        patch(id, { problem: toProblem(err) })
      }
      await reload(id)
    },
    [patch, reload],
  )

  /** Calls `onAppTurnedOn` when Repo Launcher itself had to be set to start at login too. */
  const toggleStartAtLogin = useCallback(
    async (id: string, onAppTurnedOn: () => void) => {
      const p = projectsRef.current.find((x) => x.id === id)
      if (!p) return
      try {
        const turnedOn = await api.setStartAtLogin(id, !p.startAtLogin)
        if (turnedOn) {
          patch(id, { notice: { key: 'login.appTurnedOn', params: { name: p.name } } })
          onAppTurnedOn()
        }
      } catch (err) {
        patch(id, { problem: toProblem(err) })
      }
      await reload(id)
    },
    [patch, reload],
  )

  const toggleSystemTerminal = useCallback(
    async (id: string) => {
      const p = projectsRef.current.find((x) => x.id === id)
      if (!p) return
      await api.setUseSystemTerminal(id, !p.useSystemTerminal).catch((err) => patch(id, { problem: toProblem(err) }))
      await reload(id)
    },
    [patch, reload],
  )

  const retry = useCallback(
    (id: string) => {
      const action = state.live[id]?.lastAction ?? 'install'
      if (action === 'launch') return launch(id)
      if (action === 'update' || action === 'repair') return reinstall(id, action)
      return install(id)
    },
    [state.live, launch, install, reinstall],
  )

  /** A starter project: add it, then go straight to its review (the safety gate). */
  const addAndReview = useCallback(
    async (input: string): Promise<Problem | null> => {
      let project: ProjectView
      try {
        project = await api.addProject(input)
      } catch (err) {
        return toProblem(err)
      }
      dispatch({ type: 'upsert', project })
      // The review dialog is titled with the name before the next render updates the list.
      projectsRef.current = [project, ...projectsRef.current.filter((p) => p.id !== project.id)]
      void install(project.id)
      return null
    },
    [install],
  )

  return {
    ...state,
    gate,
    keys,
    approveGate,
    closeGate: () => setGate(null),
    openKeys,
    saveKeys,
    closeKeys: () => setKeys(null),
    add,
    addAndReview,
    install,
    cancel: (id: string) => api.cancelInstall(id),
    launch,
    open: (id: string) => api.openProject(id).catch((err) => patch(id, { problem: toProblem(err) })),
    stop,
    remove,
    retry,
    update: (id: string) => reinstall(id, 'update'),
    repair: (id: string) => reinstall(id, 'repair'),
    openConsole: (id: string) => api.openConsole(id).catch((err) => patch(id, { problem: toProblem(err) })),
    toggleShortcut,
    toggleStartAtLogin,
    toggleSystemTerminal,
    /** A problem from a launch started outside the window (a shortcut, login). */
    showProblem: (id: string, problem: Problem) => patch(id, { problem, lastAction: 'launch' }),
    dismissNotice: (id: string) => patch(id, { notice: null }),
    dismissProblem: (id: string) => patch(id, { problem: null }),
  }
}

export type Projects = ReturnType<typeof useProjects>
