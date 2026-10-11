import { useCallback, useEffect, useReducer } from 'react'
import type { Problem } from '../bindings/Problem'
import type { ProjectStatus } from '../bindings/ProjectStatus'
import type { RunState } from '../bindings/RunState'
import type { Stage } from '../bindings/Stage'
import type { StepLabel } from '../bindings/StepLabel'
import { api, toProblem, type ProjectView } from '../lib/api'

export type Busy = 'install' | 'launch' | 'stop' | 'remove' | 'update'
export type Action = 'install' | 'launch' | 'update' | 'repair'

export interface Live {
  progress?: { stage: Stage; fraction: number; label: StepLabel | null }
  runState?: RunState
  problem?: Problem | null
  busy?: Busy
  /** What to repeat when the user clicks "Try again". */
  lastAction?: Action
  /** A short, non-error message such as "You have the latest version." */
  notice?: 'upToDate' | 'updated' | null
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

  const install = useCallback(
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

  const launch = useCallback(
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
          patch(id, { notice: updated ? 'updated' : 'upToDate' })
        } else {
          dispatch({ type: 'status', id, status: 'installing' })
          await api.repairProject(id)
        }
      } catch (err) {
        patch(id, { problem: toProblem(err) })
      } finally {
        patch(id, { busy: undefined, progress: undefined })
        await refresh()
      }
    },
    [patch, refresh],
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

  return {
    ...state,
    add,
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
    dismissNotice: (id: string) => patch(id, { notice: null }),
    dismissProblem: (id: string) => patch(id, { problem: null }),
  }
}

export type Projects = ReturnType<typeof useProjects>
