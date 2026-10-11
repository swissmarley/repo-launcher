// The only module that talks to the engine. Outside Tauri (plain browser, UI tests) it falls
// back to an in-memory mock so the interface can be developed and tested without Rust.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { EngineEvent } from '../bindings/EngineEvent'
import type { LaunchInfo } from '../bindings/LaunchInfo'
import type { Problem } from '../bindings/Problem'
import type { Project } from '../bindings/Project'
import { mockEngine } from './mockEngine'

export type ProjectView = Project & { running: boolean; url: string | null }

export interface EngineApi {
  listProjects(): Promise<ProjectView[]>
  addProject(input: string): Promise<ProjectView>
  installProject(id: string): Promise<ProjectView>
  cancelInstall(id: string): Promise<void>
  launchProject(id: string): Promise<LaunchInfo>
  openProject(id: string): Promise<void>
  stopProject(id: string): Promise<void>
  uninstallProject(id: string): Promise<void>
  projectLog(id: string): Promise<string[]>
  onEvent(handler: (e: EngineEvent) => void): Promise<() => void>
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const tauriApi: EngineApi = {
  listProjects: () => invoke('list_projects'),
  addProject: (input) => invoke('add_project', { input }),
  installProject: (id) => invoke('install_project', { id }),
  cancelInstall: (id) => invoke('cancel_install', { id }),
  launchProject: (id) => invoke('launch_project', { id }),
  openProject: (id) => invoke('open_project', { id }),
  stopProject: (id) => invoke('stop_project', { id }),
  uninstallProject: (id) => invoke('uninstall_project', { id }),
  projectLog: (id) => invoke('project_log', { id }),
  onEvent: (handler) => listen<EngineEvent>('engine', (e) => handler(e.payload)),
}

export const api: EngineApi = isTauri ? tauriApi : mockEngine

/** Commands reject with a serialized `Problem`; anything else becomes an internal problem. */
export function toProblem(err: unknown): Problem {
  if (err && typeof err === 'object' && 'code' in err) return err as Problem
  return { code: 'INTERNAL', params: {}, fix: { kind: 'retry' }, detail: String(err) }
}
