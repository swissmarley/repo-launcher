# Repo Launcher — Phase 0 Plan

> Status: **draft for approval** · 2026-10-11
> Working product name: **Repo Launcher** (see Open Question 1 on using "GitHub" in the name).

---

## 1. Product principles

1. **No account for public repos.** Any public GitHub repo can be added, installed, launched,
   updated and removed anonymously. GitHub sign-in is offered *only* when it's needed: a private
   repo URL, or browsing "Your projects".
2. **One primary action per screen.** Add → Install → Launch. Everything else lives behind a menu
   or an "Advanced" disclosure.
3. **No jargon.** The UI never says clone, venv, PATH, port, token, dependency, or npm.
   The engine speaks in error *codes*; the UI turns codes into human sentences.
4. **Never touch the system.** No system Python/Node/git, no admin rights, no global installs,
   and nothing written outside the app's own folders (except shortcuts the user asked for).
5. **Honest about limits.** "Works with every public repo" means every public repo can be
   *added and inspected*. Repos the engine can't run get a plain explanation instead of a
   cryptic failure (see §6.4 ecosystem tiers).

---

## 2. Stack decision — **Tauri 2 (Rust core) + React/TypeScript/Tailwind**

**Chosen: Tauri 2.** Reasons, in order of weight:

- **The hard parts are systems work**: Windows Job Objects, POSIX process groups, PTYs, OS
  keychains, checksum-verified downloads, archive extraction. These are first-class in Rust.
  In Electron each of them is a native Node module (node-pty, a keychain addon) with its own
  prebuild/ABI pain — more native complexity, not less.
- **Footprint**: the app sits in the tray while projects run. A ~10–15 MB installer and a
  small resident process suit a utility far better than a ~150 MB Electron bundle.
- **Headless engine**: a plain Rust library crate with no Tauri dependency is trivially
  testable and reusable by a CLI (and is how shortcuts and the "open in my terminal" mode work).

**Accepted trade-off**: rendering differs across WebKit (macOS), WebView2 (Windows) and
WebKitGTK (Linux). Mitigation: conservative CSS (no bleeding-edge features), a small
component set, visual checks on all three OSes, and known WebKitGTK workarounds
(e.g. `WEBKIT_DISABLE_DMABUF_RENDERER` on affected GPUs). Our UI is calm and simple, so this
risk is low.

**Frontend**: React + TypeScript + Vite + Tailwind CSS, Radix primitives for accessible
dialogs/menus/tooltips, i18next for strings, xterm.js for the built-in console.
**Local state**: SQLite via `rusqlite` (bundled). **Logging**: `tracing` (structured JSON files).

### 2.1 Key dependencies (versions verified on registries 2026-10-11; all MIT/Apache-compatible)

| Concern | Choice | Notes |
|---|---|---|
| App shell | `tauri` 2.12 + plugins: single-instance, deep-link, updater, autostart, opener, dialog, notification, window-state, log | Tray is built into core (`tray-icon`). Tauri 3 is alpha → stay on 2.x |
| Async / HTTP | `tokio` 1.53, `reqwest` 0.13 (rustls is the default TLS) | |
| Process trees | `process-wrap` 10 (tokio frontend + `JobObject` + `KillOnDrop` on Windows, `ProcessGroup` on POSIX) | Kill-on-close only with the tokio frontend + KillOnDrop |
| PTY console | `portable-pty` 0.9 + `@xterm/xterm` 6 | ConPTY on Windows |
| Keychain | `keyring-core` 1.0 + `apple-native-keyring-store`, `windows-native-keyring-store`, `zbus-secret-service-keyring-store` (pure Rust) | Windows credential blobs ≤ 2560 bytes — fine for tokens/API keys; larger values rejected with a clear message |
| State | `rusqlite` 0.40 (bundled) + `PRAGMA user_version` migrations | |
| Archives / hashing | `tar`, `flate2`, `zip`, `sha2` | Node is fetched as `.tar.gz`/`.zip`, so no xz dependency |
| Paths / processes / ports | `dirs` 7, `dunce`, `sysinfo` 0.39 (descendants via `parent()` scan), `listeners` 0.6 | `directories` is archived upstream → `dirs` |
| Shortcuts | Windows: `IShellLinkW` via Microsoft's `windows` crate; macOS: `.app` + `Info.plist` via `plist`; Linux: `.desktop` text | `.lnk` writer crates are unmaintained/unstable |
| Logging | `tracing`, `tracing-subscriber`, `tracing-appender` | JSON lines + redaction layer |
| Types → TS | `ts-rs` 12 | |
| UI | React 19, Vite 8, Tailwind 4, `radix-ui`, i18next / react-i18next, Vitest, Playwright | TypeScript pinned to the version Vite/ts tooling supports at Phase 1 |
| Git | **none** — GitHub archives + API; `gix` rejected (can't check out submodules), `git2` rejected (C build, GPL-with-exception) | |

---

## 3. Architecture

```
┌─────────────────────────── Repo Launcher (one process) ───────────────────────────┐
│  UI (React, WebView)                                                              │
│    Library · Add · Install progress · Console · Settings · Problem cards          │
│        │  typed invoke() commands            ▲  typed event channel               │
│  ──────┼─────────────────────────────────────┼──────────────────────────────────  │
│  app shell (Tauri): commands = thin adapters, tray, single-instance, deep links,  │
│                     updater, window mgmt. No business logic.                      │
│        │                                     │                                    │
│  ──────▼─────────────────────────────────────┴──────────────────────────────────  │
│  launcher-core (Rust library, no Tauri dependency, async/tokio)                   │
│   source ─► detect ─► recipe ─► safety ─► install ─► run/supervise                │
│   runtime (uv, Python, Node, pnpm/yarn, bun)   secrets (keychain)   store (SQLite)│
│   env (filtered env)   shortcuts   diagnose (error → plain language)   events     │
└───────────────────────────────────────────────────────────────────────────────────┘
        │ spawns (process group / job object, filtered env, project cwd)
        ▼
   project processes  (streamlit, node, python cli, …)  ──►  browser / built-in console
```

**One process owns all running projects.** Shortcuts and "Launch at login" start the same
binary in *background mode* (tray only, no main window) via a deep link / `--launch <id>`;
if it's already running, the single-instance plugin forwards the request. This keeps
supervision, logs, and Stop/Restart in one place.

### 3.1 Repository layout

```
Cargo.toml                 workspace
crates/
  launcher-core/           the engine (all logic, headless-testable)
  launcher-cli/            `rl` dev/test CLI: add | detect | install | run | uninstall
app/                       Tauri shell (src-tauri) — commands, tray, plugins
ui/                        React app (Vite)
fixtures/
  corpus.toml              ≥25 real public repos pinned to commit SHAs + expectations
  corpus/<owner>__<repo>/  snapshot of manifest files + file tree (for hermetic unit tests)
  synthetic/               tiny hand-made repos for edge cases (port conflicts, crash loops…)
schema/launcher.schema.json
docs/                      PLAN, DECISIONS, SECURITY, QA, THIRD_PARTY, LAUNCHER_JSON
.github/workflows/         ci.yml (3-OS matrix), integration.yml, release.yml
```

### 3.2 `launcher-core` module boundaries

| Module | Responsibility | Depends on |
|---|---|---|
| `source` | Parse input (URL / `owner/repo` / `.git` / `/tree/<branch>`), GitHub metadata, download archive, extract safely, submodules, LFS pointers, file manifest for updates | `store`, `secrets` (token, optional) |
| `detect` | Scan a source tree → `DetectionReport` (ecosystems, frameworks, entry points, env-var hints, README commands) | — (pure) |
| `recipe` | `launcher.json` parse/validate, heuristics → `Recipe`, merge/override, AI proposal validation | `detect` (pure) |
| `safety` | Risk signals, plain-language "what this will do" summary, consent records, trusted publishers | `recipe`, `source` metadata |
| `runtime` | Provision pinned, checksum-verified runtimes into the app folder; resolve tool names → managed binaries | `store` |
| `install` | Turn a `Recipe` into ordered, fingerprinted steps; run with progress, cancel, resume | `runtime`, `env` |
| `run` | Spawn/supervise process trees, ports, readiness, log streaming, PTY console, auto-restart, crash reconciliation | `env`, `store` |
| `env` | Build the filtered child environment (allowlist + managed paths + injected secrets) | `secrets` |
| `secrets` | `SecretStore` trait; OS-keychain impl; in-memory impl for tests | — |
| `shortcuts` | `.lnk` / `.desktop` / macOS `.app` wrapper create/remove/repair | `store` |
| `diagnose` | Map failures + log tails → `Problem { code, params, fix }`; build redacted diagnostics bundle | — |
| `store` | SQLite schema, migrations (`PRAGMA user_version`), repositories | — |
| `events` | `EngineEvent` stream (progress, log lines, state changes) | — |

`detect` and `recipe` are pure functions over a file tree → fast, exhaustive unit tests.

### 3.3 Engine ↔ UI contract

- Commands return `Result<T, Problem>`. A `Problem` carries a stable `code`
  (e.g. `NEEDS_COMPILER`, `PORT_IN_USE`, `REPO_NOT_FOUND_OR_PRIVATE`), interpolation
  `params`, a primary `FixAction` (`Retry`, `Repair`, `SignIn`, `EnterKey{name}`,
  `InstallMacCommandLineTools`, `OpenUrl{url}`, `UseDifferentPort`, `FreeUpSpace`…) and a
  diagnostics id. The UI owns all wording (localizable); the engine never sends English.
- Long operations stream `EngineEvent`s over a Tauri channel: `Progress{stage, fraction, detail}`,
  `Log{stream, line}`, `ProjectState{id, state}`.
- Rust types → TypeScript types are generated with `ts-rs` (stable) at build time; the thin
  `invoke` wrappers are hand-written in one file. (`tauri-specta` would also generate command
  bindings but is still a release candidate.)
- Tauri capabilities: the WebView can call **only** our own commands — no shell, fs, or http
  plugin access from JavaScript. Strict CSP.

---

## 4. Core flows → engine behavior

### 4.1 Add
1. Parse input. Accepted: `https://github.com/o/r`, `…/r.git`, `…/tree/<ref>[/subdir]`,
   `git@github.com:o/r.git`, `o/r`, plus trailing slashes/whitespace/`www.`. Anything else →
   friendly "That doesn't look like a GitHub project link" with an example.
2. Fetch metadata (name, description, stars, license, default branch, created/pushed dates,
   archived) — **the only anonymous REST call per add** (60/hour/IP budget), cached.
   A 404 while signed out → *"This project is private or doesn't exist"* + **Connect GitHub**.
   Quota-free paths are used for everything else:
   - source: `github.com/<o>/<r>/archive/<ref>.tar.gz` (doesn't touch the API quota);
     the commit SHA is read from the archive's pax header, with an API fallback;
   - README / manifest files: raw content host;
   - update checks: Git's public smart-HTTP ref listing (`/info/refs`) parsed in Rust —
     no git binary, no API quota;
   - icons: `launcher.json` icon → icon/logo files in the repo → owner avatar → generated monogram.
3. Fetch README + manifest files via raw content (not the REST API) and run `detect` on the
   partial tree **before** downloading anything, so the preview can say what will happen:
   > *"Downloads about 4 MB. Sets up Python 3.12 just for this project and installs 38
   > helpers (~450 MB). When you click Launch it opens in your web browser."*
4. Browse/search: curated "Starter projects" (embedded list) + GitHub search; when signed in,
   a "Your projects" tab.

### 4.2 Install (stages shown to the user)
| UI stage | Engine steps |
|---|---|
| **Downloading** | archive download (resumable for runtimes), safe extract, submodules, LFS pointers |
| **Preparing** | provision runtimes (uv, Python, Node…), create isolated environment |
| **Installing helpers** | dependency install (uv / npm / pnpm / yarn / bun), build steps |
| **Ready** | verification (entry point exists, imports resolve), record recipe hash |

- Every step has a **fingerprint** (hash of its inputs). Completed steps are recorded; *Resume*
  skips steps whose fingerprint is unchanged. *Cancel* propagates a cancellation token and kills
  the step's process tree; the project stays in a resumable "Not finished" state.
- Package-manager caches live in the app folder, so retries are fast.
- Progress is stage-weighted; downloads are determinate, package installs show calm activity
  text ("Still working — the first install can take a few minutes").

### 4.3 Launch
- **web**: allocate port → spawn → watch output for URLs → readiness probe → open default browser.
- **terminal**: built-in console window (real PTY + xterm.js) or the OS terminal (per-project
  setting). If a CLI exits immediately (needs arguments), the console stays open with
  "Run again" and an "Options" box.
- **gui**: spawn with no console window; track Running/Stopped.
- Card shows **Running / Starting / Stopped / Problem** with Stop, Restart, Open.

### 4.4 Shortcut
Native launcher that invokes the Launcher engine in background mode (§7.4).

### 4.5 Manage
- **Update**: download new archive to staging → apply via the stored file manifest: files we
  installed and the user didn't change are replaced/removed; files the user changed are backed up
  then replaced; files the project created at runtime (outputs, models, `.env`) are untouched.
  If the recipe changed (new install step, new run command) → consent again.
- **Repair**: delete and rebuild the environment (source and user data kept).
- **Uninstall**: stop → remove project folder, environment, logs, shortcuts, keychain entries
  (enumerated from the DB index), DB rows. Unused runtimes are garbage-collected.
- **Logs**: per-run install/run logs with "Copy diagnostics".

### 4.6 Private repos & keys
- **Connect GitHub** via OAuth device flow: big code + "Copy code & open GitHub" button →
  poll → "Connected as @name". Token in keychain. Disconnect deletes it (and links to GitHub's
  page to revoke, since revocation needs a server-side secret we don't have).
- **Keys form** when a project needs env vars: label, why it's needed, "Where do I get this?"
  link (built-in catalog for common keys: OpenAI, Anthropic, Hugging Face, Google/Gemini,
  Groq, Replicate…), stored in keychain, injected at launch only.

---

## 5. `launcher.json` schema (v1)

Lives at the repo root. Commands are **argv arrays** (no shell strings) so quoting works
identically on every OS. Full reference will be `docs/LAUNCHER_JSON.md` + `schema/launcher.schema.json`.

```jsonc
{
  "$schema": "https://…/launcher.schema.json",
  "version": 1,                                  // required
  "type": "web",                                 // required: web | terminal | gui
  "name": "Photo Restorer",                      // optional display overrides
  "description": "Fix old photos in your browser.",
  "icon": "assets/icon.png",                     // repo-relative PNG/SVG

  "requires": {
    "python": ">=3.10,<3.13",                    // PEP 440 specifier
    "node": ">=20",                              // semver range
    "packageManager": "pnpm@9",                  // npm | pnpm | yarn | bun [@version]
    "system": ["ffmpeg"]                         // checked, never auto-installed in v1
  },

  "install": [                                   // ordered steps
    { "name": "Install Python helpers", "python": { "requirements": ["requirements.txt"] } },
    { "name": "Install website helpers", "node": { "cwd": "web" } },
    { "name": "Build the website", "run": ["npm", "run", "build"], "cwd": "web" },
    { "name": "Download the model", "run": ["python", "scripts/fetch_model.py"],
      "platforms": ["macos", "linux"], "timeoutMinutes": 30 }
  ],

  "run": {
    "command": ["python", "app.py", "--port", "{port}"],
    "cwd": ".",
    "env": { "APP_MODE": "production" }
  },

  "web": {                                       // only for type=web
    "port": 7860,                                // preferred; engine picks another if busy
    "portEnv": "GRADIO_SERVER_PORT",             // alternative to {port} in argv
    "path": "/",                                 // opened in the browser
    "readyTimeoutSeconds": 180,
    "openBrowser": true
  },

  "env": [
    { "name": "OPENAI_API_KEY", "label": "OpenAI API key",
      "description": "Used to write captions.", "required": true, "secret": true,
      "helpUrl": "https://platform.openai.com/api-keys" }
  ],

  "platforms": ["macos", "windows", "linux"],
  "overrides": { "windows": { "run": { "command": ["python", "app.py", "--no-reload"] } } }
}
```

**Command resolution** (first argv element): `python`, `pip`→`uv pip`, `uv`, `node`, `npm`,
`npx`, `pnpm`, `yarn`, `bun` → the project's managed runtimes; `./path` → project-local script;
other names → must exist in the project environment (`.venv/bin`, `node_modules/.bin`) or be
declared in `requires.system`. **Rejected at validation**: `sudo`/`su`/`doas`/`runas`,
absolute paths outside the project, shell metacharacter pipelines.
**Placeholders**: `{port}`, `{host}` (127.0.0.1 unless the user allows network access),
`{projectDir}`, `{dataDir}` (persistent per-project data that survives updates).

**Resolution order**: `launcher.json` → heuristics → (opt-in) AI proposal → user picks from
candidates. Every field records its provenance so the consent screen can say where it came from.

### 5.1 Heuristics (examples, not exhaustive)
- **Python install**: `uv.lock` → `uv sync --frozen` into the external env; `pyproject.toml` /
  `setup.py` → `uv pip install -e .`; `requirements*.txt` → `uv pip install -r`; `Pipfile` →
  converted to requirements; Python version from `requires-python` / `.python-version` /
  `runtime.txt`, default 3.12.
- **Python run**: Streamlit (`streamlit run X --server.port {port} --server.address {host}
  --server.headless true --browser.gatherUsageStats false` — headless also suppresses
  Streamlit's first-run email prompt, which would otherwise hang the launch),
  Gradio (`GRADIO_SERVER_PORT`/`GRADIO_SERVER_NAME`; if the code pins `server_port`, discover
  mode), Flask, FastAPI (uvicorn), Django
  (`migrate` + `runserver`), notebooks-only (JupyterLab with a random token), console scripts
  (`[project.scripts]` → terminal), `__main__.py`.
- **Node**: package manager from `packageManager` field / lockfile; frozen install with
  non-frozen fallback; `dev`/`start` script by framework — Next (`-p {port} -H {host}`; Next
  binds all interfaces by default), Vite (`--port {port} --strictPort --host {host}`), Astro,
  Nuxt, SvelteKit, CRA (`PORT`, `BROWSER=none`), Express (`PORT`, else discover);
  flags appended after `--` when going through a package script; `bin` → terminal;
  Electron → gui; workspaces → pick the app package.
- **README mining**: fenced blocks under *Usage / Run / Quick start* headings yield scored
  candidate commands.
- **Hardcoded ports**: if the code pins a port, switch to *discover* mode (§7.2).
- **Low confidence**: show 2–4 plain-language candidates ("Start the website", "Run the
  command-line tool") instead of guessing silently.

---

## 6. Runtimes, isolation, platforms

### 6.1 Managed runtimes (app-owned, pinned, checksum-verified)
- **uv** 0.13 standalone binary (per-asset `.sha256` published) → manages **Python** builds
  (`UV_PYTHON_INSTALL_DIR` in the app folder, `UV_PYTHON_PREFERENCE=only-managed`) and
  per-project venvs placed *outside* the source tree, so updates never clobber them.
  Without a `uv.lock` we use `uv venv` + `uv pip install` (`uv sync` would write a lockfile
  into the repo); with one, `uv sync --frozen`.
  **We always pass an explicit Python version**: uv's own default is now 3.15 (released two
  days ago, thin wheel coverage). Default **3.12**, honoring `requires-python` etc.
  ML projects get `UV_TORCH_BACKEND=auto` (CPU/GPU wheel choice; preview feature in uv).
- **Node**: official binaries for the supported LTS lines (22, 24, 26 once LTS on 2026-10-28),
  hashes taken from `SHASUMS256.txt`. `engines.node` ranges map to the nearest pinned line.
- **pnpm / yarn**: corepack is gone from Node 25+, so we fetch package-manager tarballs from the
  npm registry and verify their `sha512` integrity: pnpm via its native `@pnpm/exe.<platform>`
  build, Yarn Classic via `yarn`, Yarn Berry via `@yarnpkg/cli-dist` (run with managed Node).
  The exact version comes from the `packageManager` field when present.
- **bun**: release zips verified against the published `SHASUMS256.txt`.
- **micromamba** (Tier 2, `environment.yml` only): static binary with published `.sha256`.
- Default versions + SHA-256 are **pinned in a runtime manifest compiled into the app**; a CI job
  bumps and re-verifies them. Project-requested package-manager versions are verified against
  registry integrity hashes. Downloads resume via HTTP Range into `.part` files.
- **No git needed**: source arrives as a GitHub archive; submodules are resolved through the API
  and downloaded the same way; LFS pointer files are detected and fetched.
- Caches (uv, npm, pnpm store, bun) are redirected into the app folder. Well-known per-user
  caches projects write to (`HF_HOME`, `TORCH_HOME`, `XDG_CACHE_HOME`, …) are redirected into
  the project folder so uninstall really removes them.

### 6.2 Filesystem layout
```
<app data>/                       macOS ~/Library/Application Support/<bundle id>
                                  Windows %LOCALAPPDATA%\RepoLauncher   Linux $XDG_DATA_HOME/repo-launcher
  state.db
  logs/                            app logs (rotating, 7 days, secrets scrubbed)
  runtimes/  uv/ python/ node/ bun/ …
  cache/     uv/ npm/ pnpm/ downloads/
  p/<id8>/                         short ids keep Windows paths short
    src/  env/  data/  home/  logs/  meta/(manifest, backups)
```

### 6.3 Cross-platform specifics
- **Windows**: short base paths; `\\?\`-aware file ops in our code; `CREATE_NO_WINDOW` for
  web/gui projects; Ctrl-Break then Job Object termination; never elevate (asInvoker); ARM64
  runtimes where available. Long-path registry toggle needs admin → guidance only.
- **macOS**: universal binary; runtime arch detected via `sysctl` (not the process arch, to
  avoid Rosetta mis-detection); missing compiler → button that opens Apple's own
  "Command Line Tools" installer (Apple prompts the user; we never use sudo).
- **Linux**: build on the oldest supported Ubuntu for glibc compatibility; AppImage + deb + rpm;
  missing system libraries → distro-aware copyable command (apt/dnf/pacman from
  `/etc/os-release`); the tray is optional (GNOME needs an extension), so nothing depends on it.

### 6.4 Ecosystem tiers (honest scope for v1)
| Tier | Ecosystems | v1 behavior |
|---|---|---|
| **1 — full, CI-tested on 3 OSes** | Python (pip/uv/poetry-core/pyproject), Node (npm/pnpm/yarn/bun), Streamlit, Gradio, Flask, FastAPI, Django, Jupyter, Vite, Next.js, Python & Node CLIs | install + launch automatically |
| **2 — supported, lighter testing** | Go, Rust (managed toolchains), conda `environment.yml` (pip-able subset, micromamba otherwise), Docker Compose **if Docker is already installed** | install + launch, clearer caveats |
| **3 — detected & explained** | Ruby, PHP, C/C++/Makefile-only, Java/Gradle, repos with no runnable entry point (libraries, docs, dotfiles) | plain-language "Launcher can't run this kind of project yet", README link, Copy diagnostics; `launcher.json` / AI can still describe it |

---

## 7. Process management & ports

### 7.1 Supervision
- **POSIX**: each project runs in its own process group/session; Stop = SIGTERM to the group →
  5 s grace → SIGKILL, plus a sweep of any descendants recorded before the kill.
- **Windows**: each project runs in a Job Object with kill-on-close (children die even if the
  Launcher crashes); Stop = Ctrl-Break → grace → terminate job.
- **Crash reconciliation**: run rows store pid + process start time. On startup, orphaned
  POSIX groups are detected and offered "Still running — Stop / Keep".
- **Sleep/wake**: state comes from process exit events, not health pings, so wake doesn't
  cause false "crashed" states.
- **Auto-restart** (opt-in per project): backoff 1→30 s; 5 crashes in 5 min → stop and show
  a Problem card with the log tail diagnosis.
- **Quit** stops all projects (with a confirmation if any are running). Closing the window
  keeps the app in the tray while projects run.
- Logs: line-split (handles `\r` progress bars, ANSI), in-memory ring buffer + size-capped
  per-run file, secret values scrubbed before anything touches disk.

### 7.2 Ports & readiness
- **Inject mode** (known framework or `launcher.json`): preferred port if free, else a free one;
  injected via argv template or env var; bind to `127.0.0.1` by default
  (Advanced: "Allow other devices on my network").
- **Discover mode** (hardcoded ports): parse output for `http://localhost:NNNN`-style URLs and
  inspect listening sockets of the process tree. If the pinned port is taken before launch →
  "Another program is using what this project needs" with *Stop it* (if it's one of ours).
- **Readiness**: HTTP probe (any status < 500 counts), backoff 250 ms→2 s, default 180 s, then
  "Still starting…" with *Keep waiting* while the process is alive; early exit → diagnosis.

---

## 8. Security model (summary — full threat model in `docs/SECURITY.md` in Phase 3)

**Honest baseline**: projects run as the user, unsandboxed. Our job is informed consent,
least privilege where we control it, and never leaking the user's secrets.

- **Credentials** only in the OS keychain (macOS Keychain, Windows Credential Manager, Linux
  Secret Service). No plain-text fallback: if Linux has no Secret Service, features needing
  keys are disabled with guidance. Secret values are wrapped in a redacting type; log and
  diagnostics writers scrub known secret values and token patterns.
- **GitHub**: device flow (no client secret shipped). Public repos never send a token.
  Preferred: a **GitHub App** with read-only *Contents* + *Metadata*, 8-hour tokens refreshed
  automatically (refresh tokens last 6 months); the user chooses which of their repos the app
  can see. The alternative, an OAuth App, can only reach private code via the `repo` scope,
  which is full read **and write** on every private repo, with non-expiring tokens.
  See Open Question 3.
- **Safety gate before the first install** (installs run code too — `setup.py`, npm lifecycle
  scripts): plain-language summary of what will be downloaded, installed and run, plus risk
  signals:
  - 🔴 *requires typed confirmation*: `curl|sh`-style pipes, `sudo`/elevation, obfuscated code
    (long encoded blobs fed to `eval`/`exec`), committed executables run by install steps,
    AI-suggested recipe.
  - 🟡 *shown prominently*: repo < 30 days old, very few stars, owner account very new,
    archived, no license, install-time scripts in the project's own manifests.
  - **Trusted publishers**: owners the user trusts skip yellow-level friction; red always shows.
  - Consent is stored per recipe hash; a changed recipe after update asks again.
- **Filtered environment**: children get an allowlist (locale, temp, display/session vars,
  OS essentials) + managed tool paths + this project's own keys. `AWS_*`, `GITHUB_TOKEN`,
  other projects' keys, etc. never reach a child.
- **Supply chain**: pinned + hashed runtimes, signed/notarized releases, signed auto-updates,
  locked Rust/JS dependencies, `cargo deny` license/advisory checks in CI.
- **Known v1 limits (documented)**: no sandbox; transitive npm/pip install scripts are not
  individually reviewed; a malicious project can read the user's files.

---

## 9. Data model (SQLite, `PRAGMA user_version` migrations)

```
projects        id (8-char), owner, repo, ref, subdir, display_name, description, icon_path,
                stars, license, is_private, status (added|installing|installed|needs_attention|
                uninstalling), installed_commit, recipe_json, recipe_source, recipe_hash,
                consented_recipe_hash, consented_at, settings_json (auto_restart, console_mode,
                allow_network, port_override, start_at_login), created_at, updated_at
install_runs    id, project_id, kind (install|update|repair), started_at, finished_at,
                outcome (ok|failed|cancelled), problem_code
install_steps   run_id, idx, key, fingerprint, status, started_at, finished_at, log_path
runs            id, project_id, pid, pgid, proc_start_time, port, url, started_at, ended_at,
                exit_code, end_reason (user_stop|exited|crashed|app_quit)
env_vars        project_id, name, label, required, secret, help_url, has_value,
                plain_value (only when secret=false)   -- secret values live in the keychain
shortcuts       project_id, kind (desktop|start_menu|applications|app_menu), path, target_version
trusted_publishers owner, added_at
runtimes        name, version, os, arch, path, sha256, installed_at, last_used_at
settings        key, value
```

**Keychain entries** (service = bundle id): `github.token`, `project.<id>.env.<NAME>`,
`ai.anthropic_key`. The DB is the index used to delete them — the keychain is never enumerated.

---

## 10. Optional AI-assisted recipes (opt-in, off by default)

- Settings toggle *"Let AI suggest how to start projects Launcher doesn't recognize."*
- Uses the user's own Anthropic API key (keychain). Before sending, shows exactly what will be
  sent: README (truncated), file names, manifest files — never secrets or `.env` files.
- The model must return a recipe matching the `launcher.json` schema (structured output).
  It then goes through the same validator and safety gate, is flagged 🔴 "Suggested by AI",
  and is never executed without explicit consent.
- Scope depends on Open Question 4.

---

## 11. UX plan

- **Library (home)**: grid of app cards — icon (repo icon → social preview → generated
  monogram), name, status dot, one big **Launch/Open** button, overflow menu.
- **Empty state**: *"Paste a link to any project on GitHub and Launcher sets it up for you."*
  + 5 curated starter projects (no keys needed, verified in CI on 3 OSes).
- **Add sheet** → preview → consent → **Install** → progress → **Launch**.
- **Problem card**: one human sentence, **Fix it** (primary), **Copy diagnostics** (secondary),
  "Show details" disclosure with the log. No stack traces in the main UI.
- **Project page**: overview, logs, settings, Advanced (recipe view, open folder, project shell).
- **Settings**: appearance, GitHub connection, AI suggestions, trusted publishers, storage &
  cleanup, launch at login, updates, "Remove all Launcher data".
- **Tray/menu bar**: running projects (Open/Stop), recent quick-launch, Open Launcher, Quit.
- **Onboarding**: no forced tour. First launch = welcome line + empty state; < 30 s to first install.
- **A11y**: full keyboard nav, visible focus, ARIA labels, WCAG AA contrast (checked by axe in
  CI), `rem`-based type, `prefers-reduced-motion`, `prefers-color-scheme`.
- **i18n**: every string in `ui/src/locales/en.json`; a pseudo-locale build catches
  hard-coded strings and truncation.

---

## 12. Test strategy

| Layer | What | Where it runs |
|---|---|---|
| **Unit (Rust)** | URL parsing (table tests), detection + recipe resolution over the **≥25-repo corpus** (snapshot tests), `launcher.json` validation, env filtering, risk scanner, diagnosis patterns, argv templating/quoting, archive extraction safety (path traversal, symlinks) | every push, 3 OSes |
| **Unit (UI)** | Vitest + Testing Library for components/state; i18n key coverage | every push |
| **Integration (synthetic)** | Local fixture repos: process-tree kill (grandchildren die), port conflict, hardcoded port discovery, readiness timeout, crash loop, cancel/resume, update preserving user files, uninstall leaves nothing, unicode/space paths, keychain round-trip | every push, 3 OSes |
| **Integration (real repos)** | ~8 corpus repos (Streamlit, Gradio, Flask, FastAPI, Vite, Next, Python CLI, Node CLI) really installed + launched; assert process alive and port responds / CLI exits 0. **Run with a PATH stripped of system git/python/node** to prove the clean-machine claim | nightly + PRs touching the engine, 3 OSes |
| **E2E UI (mocked)** | Playwright against the UI in a browser with a mocked engine: every core flow, error states, keyboard-only navigation, axe a11y scan | every push, 3 OSes |
| **E2E app (real)** | WebdriverIO + `@wdio/tauri-service` with its embedded WebDriver plugin (works on macOS, Windows, Linux). The plugin is compiled in **only** behind an `e2e` Cargo feature, never in release builds. Flow: paste a local fixture repo → consent → install → launch → port responds → stop → uninstall | every push, 3 OSes |
| **Manual** | `docs/QA.md`: shortcuts on each OS, keychain prompts, notarization/SmartScreen, tray on GNOME/KDE, sleep/wake, screen readers, clean-VM definition-of-done run | each release |

**Corpus**: `fixtures/corpus.toml` pins each repo to a commit SHA with expected
ecosystem/type/run command/port strategy; `cargo xtask corpus-sync` refreshes the snapshots.
**Success metric**: ≥ 90% of Tier-1 corpus repos install + launch with zero input; 100% of the
rest end in a specific, plain-language Problem (never a generic failure).

---

## 13. Risks

| # | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| 1 | Repos are wildly heterogeneous; heuristics miss | High / High | Tiers, corpus-driven development, candidate picker, `launcher.json`, AI fallback, success metric tracked in CI |
| 2 | Native builds need compilers (no wheel / node-gyp) | High / Medium | Prefer Python versions with wheel coverage; detect error signatures → guided fix (macOS CLT button; Windows Build Tools / Linux packages guidance) |
| 3 | Anonymous GitHub API limits (60 REST calls/hour/IP; search 10/minute) | Medium / High | One REST call per add; archives, raw files and update checks use quota-free endpoints; metadata cached; search debounced with a friendly "try again in a minute"; sign-in raises limits but is never required |
| 4 | Huge ML installs (torch, CUDA) | Medium / Medium | Size warning in summary, disk-space check, CPU/GPU wheel selection |
| 5 | Process escapes group on POSIX (daemonizing children) | Low / Medium | Descendant sweep, crash reconciliation; documented |
| 6 | Real-app E2E tooling is young (`@wdio/tauri-service` 1.x); official `tauri-driver` still lacks macOS | Medium / Low | Mocked-engine Playwright covers flows regardless; engine integration tests carry the real behavior; manual QA |
| 6b | `portable-pty` last published Feb 2025 (fixes exist upstream, unreleased) | Medium / Medium | Pin a reviewed upstream git revision if a needed fix is unreleased; the console is isolated behind one module |
| 7 | Signing/notarization needs paid accounts | Certain / High for DoD | Open Question 2; unsigned dev builds until then |
| 8 | Linux variance (WebKitGTK, Wayland/NVIDIA, tray, Secret Service) | Medium / Medium | Workarounds, tray optional, explicit no-keychain mode |
| 9 | AppImage moves break shortcuts | Medium / Low | Shortcut repair on startup when the binary path changes |
| 10 | Windows long paths & Defender slowness | Medium / Medium | Short base dir, guidance; documented |
| 11 | Secrets read from files (e.g. Streamlit `secrets.toml`) | Medium / Low | v1 does not write secret files (keychain-only rule); explained in UI; revisit |
| 12 | Trademark ("GitHub" in product name) | Medium / High | Open Question 1; never use GitHub logos as our icon |
| 13 | Scope creep | High / High | Tiers + "no plugin system/cloud/accounts" rule |
| 14 | Dependencies fetched *via git* (`git+https://…` in requirements, `github:` in package.json) need a git binary — and on a clean Mac, calling `/usr/bin/git` pops Apple's tools installer | Medium / Medium | Rewrite GitHub git-URLs to archive URLs before install (pip/uv and npm accept archive URLs); Windows: optional checksum-verified MinGit in the app folder; macOS/Linux: explain + guided fix |

---

## 14. Milestones

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **0 Plan** | This doc, `DECISIONS.md` | Approved |
| **1 Walking skeleton** | Workspace + UI scaffold; URL parser; archive download/extract; Python detection; uv provisioning (pinned + hashed); venv + install; run + live log in UI; SQLite basics; `rl` CLI; **CI matrix (macOS/Windows/Ubuntu)**: fmt, clippy, tests, UI typecheck/test, app build | A Streamlit app and a Python CLI install + run from the UI on my Mac; same flow green headlessly in CI on all 3 OSes |
| **2 Engine breadth** | Node + pnpm/yarn/bun; web launch (ports, discovery, readiness, browser); terminal apps (PTY console + OS terminal); multi-step recipes; `launcher.json` + schema + docs; framework heuristics; update/repair/uninstall; cancel/resume; process-tree kill; diagnosis catalog v1; **25+ repo corpus** + real-repo integration tests; Tier-2 ecosystems | Corpus success metric met on 3 OSes in CI |
| **3 Credentials & safety** | Keychain; GitHub device flow + private repos + "Your projects"; keys form + catalog; safety gate, risk signals, trusted publishers; secret scrubbing; `SECURITY.md`; AI fallback (if in scope) | Private repo flow works; secrets never on disk (asserted by tests grepping logs/state) |
| **4 Shortcuts & tray** | `.lnk` / `.desktop` / macOS `.app` wrapper; deep links + single instance + background mode; tray/menu bar; launch at login; per-project start-at-login | Shortcut cold-start launches a web project without the main window, on 3 OSes |
| **5 Polish & ship** | Onboarding, empty state + curated projects, motion, a11y pass, pseudo-locale, icons; installers (dmg, msi + exe, AppImage/deb/rpm); signing/notarization docs + release workflow; signed auto-update; `THIRD_PARTY.md`; `QA.md` | Definition of done verified on clean VMs of all 3 OSes |

After every phase: tests green, app run by me, logs checked, short report, commit.

---

## 15. Open questions (each changes the plan; **defaults apply if you just say "approved"**)

1. **Name and app ID.** GitHub's developer terms forbid names "confusingly similar" to its
   marks, so "GitHub Repo Launcher" is a risk. The app ID determines the keychain service and
   data-folder names, which are painful to change later.
   *Default:* **Repo Launcher** (descriptor "for GitHub projects"), app ID `app.repolauncher.desktop`.
2. **Where the code lives (needed for CI in Phase 1).**
   *Default:* I create a **public** GitHub repo under your `gh` account. Public repos get free
   hosted runners on all three OSes; a private repo bills your Actions minutes, and macOS
   minutes are the most expensive.
3. **GitHub sign-in type.**
   *Default:* **GitHub App**: read-only, expiring tokens, and the user picks which repos it can
   see. The cost is one extra "choose repositories" screen on GitHub, and org repos may need
   an admin's approval. Alternative: an OAuth App, which is one click simpler but gets
   read/write access to all private repos.
   Either way, you register it under your account (I'll write the steps; it only needs a
   public client ID, no secret).
4. **AI-assisted recipes in v1.**
   *Default:* **include it**, late in Phase 3: opt-in, using the user's own Anthropic API key,
   never executed without the safety gate. Alternative: defer it until after v1 to ship sooner.
5. **Signing accounts for the definition of done.** Without an Apple Developer ID
   (notarization), macOS blocks a downloaded app until the user approves it in System
   Settings, which breaks "no documentation needed". Windows shows SmartScreen warnings
   without signing.
   *Default:* everything is wired for signing through CI secrets. Builds stay unsigned until
   you provide an Apple Developer Program membership and an Azure Artifact Signing account
   (or another certificate), which is needed by Phase 5.
