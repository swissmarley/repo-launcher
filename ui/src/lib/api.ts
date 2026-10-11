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
import type { Shortcut } from '../bindings/Shortcut'
import type { Starter } from '../bindings/Starter'
import { mockEngine } from './mockEngine'

export type ProjectView = Project & { running: boolean; url: string | null; shortcuts: Shortcut[] }

/** Messages from the app shell (shortcuts, the tray, starting at login) for the main window. */
export type ShellMessage =
  | { type: 'launchProblem'; projectId: string; problem: Problem }
  /** A shortcut asked to launch a project that needs the keys form first. */
  | { type: 'launchInUi'; projectId: string }
  /** Quit was chosen while projects are running. */
  | { type: 'quitRequested' }
  /** Projects left running when Repo Launcher closed unexpectedly; they were stopped. */
  | { type: 'orphans'; projectIds: string[] }
  /** The daily check found a newer version of Repo Launcher. */
  | { type: 'updateAvailable'; version: string }

export interface ShellSettings {
  launchAtLogin: boolean
  platform: 'macos' | 'windows' | 'linux' | (string & {})
}

export interface UpdatesInfo {
  /** This copy can update itself (official releases); other builds can't. */
  configured: boolean
  autoCheck: boolean
  currentVersion: string
  /** A newer version found by the last check, ready to install. */
  available: string | null
}

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
  /** Creates (or repairs) the project's shortcuts. */
  createShortcut(id: string): Promise<Shortcut[]>
  removeShortcut(id: string): Promise<void>
  /** Resolves true if Repo Launcher itself was also set to start at login. */
  setStartAtLogin(id: string, enabled: boolean): Promise<boolean>
  setUseSystemTerminal(id: string, enabled: boolean): Promise<void>
  shellSettings(): Promise<ShellSettings>
  setLaunchAtLogin(enabled: boolean): Promise<void>
  takeMessages(): Promise<ShellMessage[]>
  /** Stops every project and quits (after the user confirmed). */
  quitApp(): Promise<void>
  /** The curated projects an empty library offers. */
  starterProjects(): Promise<Starter[]>
  /** The project's own icon as a `data:` URL, if it has one. */
  projectIcon(id: string): Promise<string | null>
  updatesInfo(): Promise<UpdatesInfo>
  /** Resolves to the newer version, if there is one. */
  checkForUpdate(): Promise<string | null>
  /** Installs the update found by the last check and restarts (running projects stop). */
  installUpdate(): Promise<void>
  setAutoUpdateCheck(enabled: boolean): Promise<void>
  /** Stops everything, removes shortcuts, keys, the login item and all files, then quits. */
  removeAllData(): Promise<void>
  /** Download progress of an update being installed, 0 to 1. */
  onUpdateProgress(handler: (fraction: number) => void): Promise<() => void>
  onEvent(handler: (e: EngineEvent) => void): Promise<() => void>
  /** Called when the shell has new messages for `takeMessages`. */
  onShell(handler: () => void): Promise<() => void>
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
  createShortcut: (id) => invoke('create_shortcut', { id }),
  removeShortcut: (id) => invoke('remove_shortcut', { id }),
  setStartAtLogin: (id, enabled) => invoke('set_start_at_login', { id, enabled }),
  setUseSystemTerminal: (id, enabled) => invoke('set_use_system_terminal', { id, enabled }),
  shellSettings: () => invoke('shell_settings'),
  setLaunchAtLogin: (enabled) => invoke('set_launch_at_login', { enabled }),
  takeMessages: () => invoke('take_messages'),
  quitApp: () => invoke('quit_app'),
  starterProjects: () => invoke('starter_projects'),
  projectIcon: (id) => invoke('project_icon', { id }),
  updatesInfo: () => invoke('updates_info'),
  checkForUpdate: () => invoke('check_for_update'),
  installUpdate: () => invoke('install_update'),
  setAutoUpdateCheck: (enabled) => invoke('set_auto_update_check', { enabled }),
  removeAllData: () => invoke('remove_all_data'),
  onUpdateProgress: (handler) => listen<number>('update-progress', (e) => handler(e.payload)),
  onEvent: (handler) => listen<EngineEvent>('engine', (e) => handler(e.payload)),
  onShell: (handler) => listen('shell', () => handler()),
}

export const api: EngineApi = isTauri ? tauriApi : mockEngine

/** Commands reject with a serialized `Problem`; anything else becomes an internal problem. */
export function toProblem(err: unknown): Problem {
  if (err && typeof err === 'object' && 'code' in err) return err as Problem
  return { code: 'INTERNAL', params: {}, fix: { kind: 'retry' }, detail: String(err) }
}
