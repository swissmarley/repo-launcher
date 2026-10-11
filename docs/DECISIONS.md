# Decisions

Short records of choices that shape the product. Newest at the bottom. When two requirements
conflict, we pick the safer, simpler option for non-technical users and record it here.

---

### D-001 · Tauri 2 (Rust core) instead of Electron — 2026-10-11
The hard parts (process trees, job objects, PTYs, keychains, verified downloads) are systems
work that Rust does natively; Electron would need several native Node modules for the same
things. Tauri also gives a far smaller installer and lower idle RAM for a tray utility.
Cost: three different WebViews. Mitigated with conservative CSS and per-OS visual checks.

### D-002 · Public repos never require a GitHub account — 2026-10-11 (product owner directive)
Adding, installing, launching, updating and removing public repos work anonymously.
"Connect GitHub" appears only for private repos and for browsing the user's own repos.
Consequence: the engine minimizes anonymous REST API calls (archive and raw-content downloads,
ETag-cached metadata) so the anonymous rate limit is rarely reached.

### D-003 · No git at all: download GitHub archives — 2026-10-11
Source code arrives as a GitHub archive at a specific commit. Submodules are resolved through
the API and downloaded the same way; Git LFS pointer files are detected and fetched.
Updates use a stored file manifest (replace unchanged files, back up user-changed files,
leave files the project created alone). `gix` was rejected because it can't check out
submodules; `git2` because it needs a C build and libgit2's license.
Known gap: tools that read version numbers from git metadata (`setuptools-scm`,
`hatch-vcs`) get a pretend version via their documented environment variables.

### D-004 · One process owns every running project — 2026-10-11
Shortcuts and launch-at-login start the same app binary in background mode (tray only) through
a deep link / `--launch <id>`. If the app is already running, the request is forwarded to it.
This keeps Stop/Restart, logs and state in one place instead of splitting supervision across
processes.

### D-005 · Projects stop when the Launcher quits — 2026-10-11
Closing the window keeps the app in the tray while projects run; *Quit* stops everything
(confirming first if anything is running). Leaving orphaned servers running is surprising
for non-technical users and keeps ports busy.

### D-006 · Consent happens before the first install, not just the first run — 2026-10-11
Installing a project already runs its code (`setup.py`, npm lifecycle scripts), so the safety
gate is shown before Install. Consent is tied to a hash of the recipe; if an update changes
what will be installed or run, we ask again.

### D-007 · Secrets live only in the OS keychain, with no plain-text fallback — 2026-10-11
If a Linux desktop has no Secret Service, features that need secrets are disabled with
guidance; public projects that need no keys keep working. Projects that read keys from a
*file* (e.g. Streamlit's `secrets.toml`) are not given one in v1; the UI explains this.
Safer beats more compatible here. Revisit if this blocks many popular projects.

### D-008 · Commands are argv arrays, never shell strings — 2026-10-11
`launcher.json` and internal recipes use argv arrays so quoting behaves identically on every
OS and no shell interprets user- or repo-supplied text. `sudo`/`su`/`doas`/`runas`, absolute
paths outside the project, and shell pipelines are rejected at validation.

### D-009 · Web projects listen on 127.0.0.1 by default — 2026-10-11
Where a framework allows it, we bind to localhost so a project isn't exposed to the local
network by accident. "Allow other devices on my network" is an Advanced, per-project toggle.

### D-010 · Ecosystem tiers for v1 — 2026-10-11
Tier 1 (Python, Node and their common web/CLI frameworks) is fully automated and CI-tested on
three OSes. Tier 2 (Go, Rust, conda, Docker-if-installed) is supported with lighter testing.
Tier 3 (Ruby, PHP, C/C++, Java, non-runnable repos) is detected and explained in plain language.
This keeps v1 shippable without pretending every repo can run.

### D-011 · Prefer a JS project's dev server for web launches — 2026-10-11
For Vite/Next-style apps without a `launcher.json`, we run the `dev` script rather than
build + serve: fewer steps and fewer failure points. A `launcher.json` can choose otherwise.

### D-012 · macOS shortcuts go to ~/Applications, not the Dock plist — 2026-10-11
We create a small `.app` in `~/Applications` (visible in Launchpad and Spotlight) and an
optional Desktop alias. We don't edit the Dock's preferences (fragile, and it restarts the
Dock); the UI tells the user they can drag the app to the Dock or choose "Keep in Dock".

### D-013 · Python environments live outside the source folder — 2026-10-11
Venvs are created in `p/<id>/env` (via `UV_PROJECT_ENVIRONMENT` / `uv venv <path>`), so
updates and repairs never collide with files from the repo.

### D-014 · Real-app E2E driver is test-only — 2026-10-11
The embedded WebDriver plugin used by `@wdio/tauri-service` is compiled in only behind an
`e2e` Cargo feature. Release builds never contain a remote-control endpoint.

### D-015 · How runtimes are verified — 2026-10-11
Default versions of uv, Node, bun and micromamba are pinned with SHA-256 hashes compiled into
the app; a CI job bumps them. Python builds are verified by uv itself. pnpm/yarn (no corepack
in Node 25+, no checksums on pnpm's GitHub binaries) come from the npm registry and are checked
against the registry's `sha512` integrity. Node versions requested by a project map to the
nearest pinned LTS line instead of downloading arbitrary versions.

### D-016 · Explicit Python version, default 3.12 — 2026-10-11
uv 0.13 defaults to Python 3.15, released 2026-10-09, and few packages have wheels for it yet.
We always pass a version: the project's constraint if it has one, otherwise 3.12 for the
broadest prebuilt-package coverage. We'll revisit once 3.13/3.14 wheel coverage is clearly
equal.

### D-017 · Turn off projects' own usage tracking where there's a documented switch — 2026-10-11
For example, `--browser.gatherUsageStats false` for Streamlit and `GRADIO_ANALYTICS_ENABLED=False`
for Gradio. Users didn't knowingly opt in to the frameworks' telemetry, and this is the
privacy-preserving default. A `launcher.json` can still set its own values.
