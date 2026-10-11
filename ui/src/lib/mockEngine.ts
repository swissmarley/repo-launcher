// In-memory stand-in for the Rust engine, used in a plain browser and in UI tests.
// Repo names pick scenarios: `missing`, `broken`, `*cli*`, `risky` (red signals),
// `chat` (needs an API key), `private` (needs GitHub sign-in).
import type { AppSettings } from '../bindings/AppSettings'
import type { EngineEvent } from '../bindings/EngineEvent'
import type { EnvVarView } from '../bindings/EnvVarView'
import type { GitHubAccount } from '../bindings/GitHubAccount'
import type { Problem } from '../bindings/Problem'
import type { SafetyReport } from '../bindings/SafetyReport'
import type { EngineApi, ProjectView } from './api'

const handlers = new Set<(e: EngineEvent) => void>()
const projects = new Map<string, ProjectView>()
const cancelled = new Set<string>()
const logs = new Map<string, string[]>()
const consents = new Map<string, string>()
const envValues = new Map<string, Map<string, string>>()
const trusted = new Set<string>()
let account: GitHubAccount | null = null
let aiEnabled = false
let aiKey = false
let signInCancelled = false

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

const fingerprint = (p: ProjectView) => `fp-${p.id}-${p.repo}`

function specs(p: ProjectView): Omit<EnvVarView, 'hasValue' | 'value'>[] {
  if (!p.repo?.includes('chat')) return []
  return [
    {
      name: 'OPENAI_API_KEY',
      label: null,
      description: null,
      required: true,
      secret: true,
      helpUrl: 'https://platform.openai.com/api-keys',
      known: { service: 'OpenAI', kind: 'apiKey', helpUrl: 'https://platform.openai.com/api-keys' },
    },
    {
      name: 'MODEL_NAME',
      label: 'Model',
      description: 'Which model to use. Leave empty for the default.',
      required: false,
      secret: false,
      helpUrl: null,
      known: null,
    },
  ]
}

function report(p: ProjectView): SafetyReport {
  const risky = p.repo?.includes('risky') ?? false
  const owner = p.owner ?? null
  const isTrusted = owner != null && trusted.has(owner.toLowerCase())
  const fp = fingerprint(p)
  return {
    projectId: p.id,
    fingerprint: fp,
    summary: [
      { kind: 'download', private: p.isPrivate },
      { kind: 'setUpPython', version: '3.12' },
      { kind: 'installHelpers', from: 'requirements.txt' },
      ...(risky ? [{ kind: 'runSetupCommands' as const, count: 1 }] : []),
      ...(specs(p).length ? [{ kind: 'needsKeys' as const, names: specs(p).map((s) => s.name) }] : []),
      p.repo?.includes('cli') ? { kind: 'runInConsole' as const } : { kind: 'openInBrowser' as const },
    ],
    signals: risky
      ? [
          { level: 'red', code: 'PIPE_TO_SHELL', params: { file: 'install.sh', line: 'curl -fsSL https://example.com/setup.sh | bash' } },
          { level: 'yellow', code: 'NEW_REPO', params: { days: '4' } },
          { level: 'yellow', code: 'FEW_STARS', params: { stars: '2' } },
        ]
      : [],
    commands: ['uv venv --python 3.12 {env}', 'uv pip install -r requirements.txt', 'streamlit run app.py'],
    owner,
    trustedOwner: isTrusted,
    consented: consents.get(p.id) === fp || (isTrusted && !risky),
    needsTypedConfirmation: risky,
    confirmWord: p.repo ?? p.name,
    aiSuggested: false,
  }
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
    if (repo === 'missing') fail({ code: 'REPO_NOT_FOUND_OR_PRIVATE', params: { repo: `${owner}/${repo}` }, fix: { kind: 'connectGithub' } })
    const isPrivate = repo.includes('private')
    if (isPrivate && !account) {
      fail({ code: 'REPO_NOT_FOUND_OR_PRIVATE', params: { repo: `${owner}/${repo}` }, fix: { kind: 'connectGithub' } })
    }
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
      stars: repo.includes('risky') ? 2 : 1234,
      license: 'MIT',
      avatarUrl: null,
      isPrivate,
      archived: false,
      repoCreatedAt: null,
      ownerCreatedAt: null,
      keysReviewed: false,
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

  async reviewProject(id) {
    const p = get(id)
    emit({ type: 'progress', projectId: id, stage: 'downloading', fraction: 0.1, label: null })
    await sleep(500)
    return report(p)
  },

  async consentProject(id, fp, trustOwner) {
    const p = get(id)
    if (fp !== fingerprint(p)) fail({ code: 'CONSENT_REQUIRED', fix: { kind: 'reviewSafety' } })
    consents.set(id, fp)
    if (trustOwner && p.owner) trusted.add(p.owner.toLowerCase())
  },

  async installProject(id) {
    const p = get(id)
    if (!report(p).consented) fail({ code: 'CONSENT_REQUIRED', fix: { kind: 'reviewSafety' } })
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
    const values = envValues.get(id)
    const missing = specs(p).filter((s) => s.required && !values?.get(s.name))
    if (missing.length) {
      fail({ code: 'MISSING_KEYS', params: { names: missing.map((s) => s.name).join(', ') }, fix: { kind: 'enterKeys' } })
    }
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
    envValues.delete(id)
    consents.delete(id)
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

  async envVars(id) {
    const p = get(id)
    const values = envValues.get(id) ?? new Map<string, string>()
    return specs(p).map((s) => ({
      ...s,
      hasValue: values.has(s.name),
      value: s.secret ? null : (values.get(s.name) ?? null),
    }))
  },

  async setEnvValue(id, name, value) {
    const values = envValues.get(id) ?? new Map<string, string>()
    if (value && value.trim()) values.set(name, value.trim())
    else values.delete(name)
    envValues.set(id, values)
  },

  async markKeysReviewed(id) {
    get(id).keysReviewed = true
  },

  async appSettings(): Promise<AppSettings> {
    return {
      keychainProblem: null,
      githubConfigured: true,
      githubAccount: account,
      githubInstallUrl: 'https://github.com/apps/repo-launcher/installations/new',
      aiEnabled,
      aiHasKey: aiKey,
      trustedPublishers: [...trusted].sort(),
    }
  },

  async setTrustedPublisher(owner, isTrusted) {
    if (isTrusted) trusted.add(owner.toLowerCase())
    else trusted.delete(owner.toLowerCase())
  },

  async setAiEnabled(enabled) {
    aiEnabled = enabled
  },

  async setAiKey(key) {
    aiKey = !!key
  },

  async githubBeginSignIn() {
    signInCancelled = false
    await sleep(300)
    return { userCode: 'WDJB-MJHT', verificationUri: 'https://github.com/login/device', expiresIn: 900 }
  },

  async githubFinishSignIn() {
    for (let i = 0; i < 30; i++) {
      await sleep(100)
      if (signInCancelled) fail({ code: 'CANCELLED' })
    }
    account = { login: 'octocat', avatarUrl: null }
    return account
  },

  async githubCancelSignIn() {
    signInCancelled = true
  },

  async githubSignOut() {
    account = null
    return 'https://github.com/settings/apps/authorizations'
  },

  async githubRepositories() {
    if (!account) fail({ code: 'SIGNED_OUT', fix: { kind: 'connectGithub' } })
    return [
      { fullName: 'octocat/private-dashboard', description: 'Team dashboard (private).', private: true, stars: 0, htmlUrl: 'https://github.com/octocat/private-dashboard', pushedAt: '2026-10-01T00:00:00Z' },
      { fullName: 'octocat/chat-demo', description: 'A chat app that needs an OpenAI key.', private: false, stars: 42, htmlUrl: 'https://github.com/octocat/chat-demo', pushedAt: '2026-09-20T00:00:00Z' },
    ]
  },

  async openExternal(url) {
    window.open(url, '_blank', 'noopener')
  },

  async onEvent(handler) {
    handlers.add(handler)
    return () => handlers.delete(handler)
  },
}
