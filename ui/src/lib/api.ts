// The only module that talks to the engine. Outside Tauri (plain browser, UI tests) it falls
// back to an in-memory mock so the interface can be developed and tested without Rust.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AiOutcome } from '../bindings/AiOutcome'
import type { AppSettings } from '../bindings/AppSettings'
import type { DeviceCode } from '../bindings/DeviceCode'
import type { EngineEvent } from '../bindings/EngineEvent'
import type { EnvVarView } from '../bindings/EnvVarView'
import type { GitHubAccount } from '../bindings/GitHubAccount'
import type { LaunchInfo } from '../bindings/LaunchInfo'
import type { Problem } from '../bindings/Problem'
import type { Project } from '../bindings/Project'
import type { Preview } from '../bindings/Preview'
import type { RemoteRepo } from '../bindings/RemoteRepo'
import type { SafetyReport } from '../bindings/SafetyReport'
import { mockEngine } from './mockEngine'

export type ProjectView = Project & { running: boolean; url: string | null }

export interface EngineApi {
  listProjects(): Promise<ProjectView[]>
  addProject(input: string): Promise<ProjectView>
  installProject(id: string): Promise<ProjectView>
  cancelInstall(id: string): Promise<void>
  launchProject(id: string, args?: string): Promise<LaunchInfo>
  openProject(id: string): Promise<void>
  stopProject(id: string): Promise<void>
  uninstallProject(id: string): Promise<void>
  projectLog(id: string): Promise<string[]>
  getProject(id: string): Promise<ProjectView>
  openConsole(id: string): Promise<void>
  consoleWrite(id: string, data: string): Promise<void>
  consoleResize(id: string, cols: number, rows: number): Promise<void>
  consoleScrollback(id: string): Promise<string>
  /** Resolves true if a newer version was installed. */
  updateProject(id: string): Promise<boolean>
  repairProject(id: string): Promise<ProjectView>
  /** Downloads the project (nothing runs) and returns what installing it would do. */
  reviewProject(id: string): Promise<SafetyReport>
  consentProject(id: string, fingerprint: string, trustOwner: boolean): Promise<void>
  envVars(id: string): Promise<EnvVarView[]>
  /** `null` clears the value. Secret values go to the system keychain. */
  setEnvValue(id: string, name: string, value: string | null): Promise<void>
  markKeysReviewed(id: string): Promise<void>
  appSettings(): Promise<AppSettings>
  setTrustedPublisher(owner: string, trusted: boolean): Promise<void>
  setAiEnabled(enabled: boolean): Promise<void>
  setAiKey(key: string | null): Promise<void>
  githubBeginSignIn(): Promise<DeviceCode>
  githubFinishSignIn(): Promise<GitHubAccount>
  githubCancelSignIn(): Promise<void>
  /** Resolves to GitHub's page where access can also be revoked. */
  githubSignOut(): Promise<string>
  githubRepositories(): Promise<RemoteRepo[]>
  /** Opens an https page in the user's browser. */
  openExternal(url: string): Promise<void>
  /** Exactly what asking AI about this project would send. */
  aiPreview(id: string): Promise<Preview>
  /** Sends the previewed message (`digest` must match) and saves a validated suggestion. */
  aiSuggest(id: string, digest: string): Promise<AiOutcome>
  onEvent(handler: (e: EngineEvent) => void): Promise<() => void>
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const tauriApi: EngineApi = {
  listProjects: () => invoke('list_projects'),
  addProject: (input) => invoke('add_project', { input }),
  installProject: (id) => invoke('install_project', { id }),
  cancelInstall: (id) => invoke('cancel_install', { id }),
  launchProject: (id, args) => invoke('launch_project', { id, args: args ?? null }),
  openProject: (id) => invoke('open_project', { id }),
  stopProject: (id) => invoke('stop_project', { id }),
  uninstallProject: (id) => invoke('uninstall_project', { id }),
  projectLog: (id) => invoke('project_log', { id }),
  getProject: (id) => invoke('get_project', { id }),
  openConsole: (id) => invoke('open_console', { id }),
  consoleWrite: (id, data) => invoke('console_write', { id, data }),
  consoleResize: (id, cols, rows) => invoke('console_resize', { id, cols, rows }),
  consoleScrollback: (id) => invoke('console_scrollback', { id }),
  updateProject: (id) => invoke('update_project', { id }),
  repairProject: (id) => invoke('repair_project', { id }),
  reviewProject: (id) => invoke('review_project', { id }),
  consentProject: (id, fingerprint, trustOwner) => invoke('consent_project', { id, fingerprint, trustOwner }),
  envVars: (id) => invoke('env_vars', { id }),
  setEnvValue: (id, name, value) => invoke('set_env_value', { id, name, value }),
  markKeysReviewed: (id) => invoke('mark_keys_reviewed', { id }),
  appSettings: () => invoke('app_settings'),
  setTrustedPublisher: (owner, trusted) => invoke('set_trusted_publisher', { owner, trusted }),
  setAiEnabled: (enabled) => invoke('set_ai_enabled', { enabled }),
  setAiKey: (key) => invoke('set_ai_key', { key }),
  githubBeginSignIn: () => invoke('github_begin_sign_in'),
  githubFinishSignIn: () => invoke('github_finish_sign_in'),
  githubCancelSignIn: () => invoke('github_cancel_sign_in'),
  githubSignOut: () => invoke('github_sign_out'),
  githubRepositories: () => invoke('github_repositories'),
  openExternal: (url) => invoke('open_external', { url }),
  aiPreview: (id) => invoke('ai_preview', { id }),
  aiSuggest: (id, digest) => invoke('ai_suggest', { id, digest }),
  onEvent: (handler) => listen<EngineEvent>('engine', (e) => handler(e.payload)),
}

export const api: EngineApi = isTauri ? tauriApi : mockEngine

/** Commands reject with a serialized `Problem`; anything else becomes an internal problem. */
export function toProblem(err: unknown): Problem {
  if (err && typeof err === 'object' && 'code' in err) return err as Problem
  return { code: 'INTERNAL', params: {}, fix: { kind: 'retry' }, detail: String(err) }
}
