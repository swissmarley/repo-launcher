// In-memory stand-in for the Rust engine, used in a plain browser and in UI tests.
import type { EngineEvent } from '../bindings/EngineEvent'
import type { Problem } from '../bindings/Problem'
import type { EngineApi, ProjectView } from './api'

const handlers = new Set<(e: EngineEvent) => void>()
const projects = new Map<string, ProjectView>()
const cancelled = new Set<string>()
const logs = new Map<string, string[]>()

const emit = (e: EngineEvent) => handlers.forEach((h) => h(e))
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
const fail = (p: Partial<Problem> & Pick<Problem, 'code'>): never => {
  throw { params: {}, fix: null, detail: null, ...p } satisfies Problem
}

function log(id: string, line: string) {
  logs.set(id, [...(logs.get(id) ?? []), line])
  emit({ type: 'log', projectId: id, stream: 'stdout', line })
}

function get(id: string): ProjectView {
  const p = projects.get(id)
  if (!p) fail({ code: 'PROJECT_NOT_FOUND' })
  return p!
}

export const mockEngine: EngineApi = {
  async listProjects() {
    return [...projects.values()].sort((a, b) => b.createdAt - a.createdAt)
  },

  async addProject(input) {
    await sleep(300)
    const m = input.trim().match(/^(?:https?:\/\/)?(?:www\.)?(?:github\.com\/)?([\w-]+)\/([\w.-]+?)(?:\.git)?\/?$/i)
    if (!m) fail({ code: 'INVALID_LINK', params: { input } })
    const [, owner, repo] = m!
    if (repo === 'missing') fail({ code: 'REPO_NOT_FOUND_OR_PRIVATE', params: { repo: `${owner}/${repo}` } })
    const now = Date.now() / 1000
    const p: ProjectView = {
      id: Math.random().toString(36).slice(2, 10),
      source: `github:${owner}/${repo}`.toLowerCase(),
      name: repo,
      description: 'A demo project used while developing the interface.',
      owner,
      repo,
      refName: 'main',
      commit: 'a'.repeat(40),
      subdir: null,
      stars: 1234,
      license: 'MIT',
      avatarUrl: null,
      status: 'added',
      kind: null,
      createdAt: now,
      updatedAt: now,
      running: false,
      url: null,
    }
    projects.set(p.id, p)
    return p
  },

  async installProject(id) {
    const p = get(id)
    cancelled.delete(id)
    p.status = 'installing'
    const steps: Extract<EngineEvent, { type: 'progress' }>[] = [
      { type: 'progress', projectId: id, stage: 'downloading', fraction: 0.1, label: null },
      { type: 'progress', projectId: id, stage: 'preparing', fraction: 0.35, label: { kind: 'setUpPython', version: '3.12' } },
      { type: 'progress', projectId: id, stage: 'preparing', fraction: 0.5, label: { kind: 'createEnvironment' } },
      { type: 'progress', projectId: id, stage: 'installingHelpers', fraction: 0.7, label: { kind: 'installHelpers', from: 'requirements.txt' } },
    ]
    for (const e of steps) {
      if (cancelled.has(id)) {
        p.status = 'added'
        fail({ code: 'CANCELLED' })
      }
      emit(e)
      log(id, `step: ${e.stage}`)
      await sleep(500)
    }
    if (p.repo === 'broken') {
      p.status = 'needsAttention'
      fail({ code: 'INSTALL_STEP_FAILED', fix: { kind: 'retry' }, detail: 'exit code 1\nerror: something failed' })
    }
    emit({ type: 'progress', projectId: id, stage: 'ready', fraction: 1, label: null })
    p.status = 'installed'
    p.kind = p.repo?.includes('cli') ? 'terminal' : 'web'
    return { ...p }
  },

  async cancelInstall(id) {
    cancelled.add(id)
  },

  async launchProject(id) {
    const p = get(id)
    emit({ type: 'runState', projectId: id, state: 'starting', url: null, exitCode: null })
    await sleep(600)
    p.running = true
    p.url = p.kind === 'web' ? 'http://127.0.0.1:8501/' : null
    log(id, 'started')
    emit({ type: 'runState', projectId: id, state: 'running', url: p.url, exitCode: null })
    return { url: p.url, port: p.url ? 8501 : null }
  },

  async openProject() {},

  async stopProject(id) {
    const p = get(id)
    p.running = false
    p.url = null
    emit({ type: 'runState', projectId: id, state: 'stopped', url: null, exitCode: null })
  },

  async uninstallProject(id) {
    projects.delete(id)
    logs.delete(id)
  },

  async projectLog(id) {
    return logs.get(id) ?? []
  },

  async getProject(id) {
    return { ...get(id) }
  },

  async openConsole() {},

  async consoleWrite(id, data) {
    // Echo like a terminal would, so the console can be tried in a browser.
    emit({ type: 'console', projectId: id, data: data === '\r' ? '\r\n' : data })
  },

  async consoleResize() {},

  async consoleScrollback() {
    return 'Hello from the mock console\r\n$ '
  },

  async updateProject() {
    await sleep(800)
    return false
  },

  async repairProject(id) {
    return mockEngine.installProject(id)
  },

  async onEvent(handler) {
    handlers.add(handler)
    return () => handlers.delete(handler)
  },
}
