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

### D-018 · Framework dev servers are started directly, not through the package manager — 2026-10-11
When a `dev` script is a single framework command (`vite`, `next dev`, `astro dev`…), we run that
binary from `node_modules/.bin` and append our port/host flags. npm, pnpm and Yarn each forward
extra arguments differently (`--` handling), so going through them would make port injection
unreliable. Scripts that need a shell (`a && b`, `concurrently …`) run through the package
manager with `PORT` set, and the real address is read from the app's output.

### D-019 · Any HTTP answer means "ready" — 2026-10-11
A web project counts as started as soon as its server answers at all, even with an error page.
Example: Flask's tutorial returns an error until its database is initialized; showing that page
(plus the activity log) helps the user more than waiting for a timeout.

### D-020 · pnpm keeps its default of not running dependencies' install scripts — 2026-10-11
pnpm 10 skips lifecycle scripts of dependencies unless a project allows them. That's the safer
default and we keep it. npm projects still run them (npm's behavior); see SECURITY.md (Phase 3).

### D-021 · Docker is used only if the user already has it — 2026-10-11
Compose-only projects run `docker compose up --build` in the console when `docker` is found;
otherwise the user is told the project needs Docker. We never install Docker: it needs admin
rights and a background service, which is beyond "an app from an app store".

### D-022 · Notebook servers get a fresh random token per launch — 2026-10-11
JupyterLab is started with a new random token each time, and the address we open already
includes it. Other programs on the computer can't use the notebook server without it.

### D-023 · Terminal programs run in a pseudo-terminal in the built-in console — 2026-10-11
Interactive prompts, colors and progress bars need a real terminal, so terminal projects use a
PTY and an xterm.js console window. "Open in my system terminal" moves to Phase 4: it needs the
app's headless mode (also used by shortcuts) so keys never have to be written into a script file.

### D-024 · The test corpus stores reduced snapshots, not other people's code — 2026-10-11
`fixtures/corpus/` keeps each repo's file listing, its manifest files, and only the source lines
detection reads (imports, app constructors, run instructions). Tests stay offline and fast, and
we don't redistribute projects' source.

### D-025 · GitHub sign-in needs only a public client ID — 2026-10-11
The GitHub App's client ID comes from `REPO_LAUNCHER_GITHUB_CLIENT_ID`, the settings database
(`rl config github-app`), or a value compiled into release builds, in that order. The device
flow and refreshing device-flow tokens need no client secret, so none is created or shipped.
Builds without a client ID still work for public projects and explain why sign-in is missing.

### D-026 · Private repositories use the API; public ones never see the token — 2026-10-11
A project is private when anonymous access can't see it but the user's token can. Only then is
the token used: API metadata (Bearer), `info/refs` (Basic, user `x-access-token`) and the API
tarball, which redirects to a short-lived codeload link (the HTTP client drops credentials on
cross-host redirects). Public projects keep the quota-free anonymous paths even when the user
is signed in. A 404 while signed out is `REPO_NOT_FOUND_OR_PRIVATE` with "Connect GitHub";
while signed in it is `REPO_NOT_SHARED` with a link to choose repositories for the app.

### D-027 · Keys are given to the running project only, not to install steps — 2026-10-11
Values are read from the keychain at Launch and placed in that process's environment. Install
steps run without them, so dependency install scripts never see the user's keys. If a project
needs a key while installing (rare: e.g. a gated model download), its author can move that step
to first run; we revisit if this blocks popular projects.

### D-028 · The consent fingerprint covers what runs, from the project's own files — 2026-10-11
`Recipe::fingerprint()` hashes the install steps, run command, runtimes and web settings, but not
the list of keys a project asks for (a new optional key doesn't change what runs). Detection
for the plan reads only files that came from the download (the sync manifest), so files created
by installing (npm's `package-lock.json`) don't change the plan and re-trigger consent on
Repair. Consent to an outdated fingerprint is refused.

### D-029 · Trusted publishers skip the gate only when nothing is red — 2026-10-11
"Trust projects from <owner>" removes the yellow-level friction entirely: with no red signals
the install starts without the dialog. Red signals always show and always need typed
confirmation, whoever published the project.

### D-030 · Detected keys don't block a launch unless the code can't run without them — 2026-10-11
Keys found by heuristics (`os.getenv`, `process.env`, `.env.example`) are optional: many projects
support several providers and need only one. They are shown once, in a form before the first
launch, with "Launch without them". Only keys a `launcher.json` marks `required`, or that the
code reads with `os.environ["X"]` (which fails when missing), block a launch with `MISSING_KEYS`.

### D-031 · One size limit for secrets on every OS — 2026-10-11
Windows Credential Manager stores at most 2560 bytes per credential. We enforce that limit
everywhere (`SECRET_TOO_LARGE`) so a key saved on a Mac also works on Windows, rather than
splitting values across entries.

### D-032 · Secrets are scrubbed where text is produced — 2026-10-11
`Problem::detail` and `ProjectLog::line` scrub their input, so no caller can forget to. The
scrubber knows every secret value the engine has handled this session plus common token
formats. The live console hides known values on screen as a best effort (a value split across
two reads can slip through on screen); its log file is always scrubbed per line.

### D-033 · AI-suggested recipes: one structured request, exactly what was shown — 2026-10-11
Off by default; uses the user's own Anthropic key from the keychain. The message is built
deterministically from the downloaded files (file names, README, manifests; `.env`, key and
credential files excluded; all text scrubbed) and shown in full before sending; the request is
sent only if its digest still matches what the user saw. One Messages API call to
`claude-opus-5-5` (raw HTTP: there is no official Rust SDK) with `output_config.format` set to a
JSON schema mirroring launcher.json v1, and `fallbacks: "default"` so a safety-classifier
decline is retried server-side. `stop_reason` is checked (refusal, truncation) before the answer
is read. The answer is parsed by the same validator as an author's own `launcher.json`, saved
as `meta/ai-launcher.json`, used only when the project has neither its own file nor a heuristic
match, and always shown as a red "suggested by AI" signal that needs typed confirmation.

### D-034 · Shortcuts start the Launcher directly on Windows and Linux, a link on macOS — 2026-10-11
Windows `.lnk` and Linux `.desktop` shortcuts run the Launcher with `--launch <id>`; nothing has
to be registered for them to work, and they are rewritten at startup if the app moves (an
AppImage). The macOS shortcut is a tiny app in ~/Applications whose script runs
`open repolauncher://launch/<id>`, so LaunchServices finds the Launcher wherever it is. Each file
is recorded in the `shortcuts` table, so removing a project (or `rl remove-shortcuts`) removes
it; a shortcut the user deleted is forgotten, never recreated. Existing files that aren't ours
are never overwritten: a numbered name is used instead.

### D-035 · Projects left running by a crash are stopped, not adopted — 2026-10-11
PLAN §7.1 offered "Still running: Stop / Keep". Tried on macOS: a project that outlived the
Launcher still runs, but its output went to the Launcher that is gone, so a server that logs each
request (Python's `http.server`, most dev servers) fails every request with a broken pipe while
looking "Running". So on the next start, runs whose process is verifiably ours (pid plus process
start time plus boot time; on POSIX the whole process group) are stopped, and the user is told
once. A pid reused by another program is never touched. A shortcut that started the Launcher
then launches a fresh, working copy. On Windows the job object's kill-on-close normally means
nothing survives a crash at all.

### D-036 · Closing the window: stay in the tray while projects run or after a login start — 2026-10-11
Closing the main window hides it while projects run (D-005), and also when the Launcher was
started at login with a working tray icon, since being there is the point. Otherwise closing it
quits, which is what people expect and what keeps the app reachable on desktops without a tray
(GNOME without an extension). Starting the Launcher again always shows the window.

### D-037 · macOS: our ⌘Q asks first; a system quit can't be held back — 2026-10-11
Tauri's windowing layer doesn't implement `applicationShouldTerminate`, so a quit coming from
macOS itself (logging out, an Apple Event, the Dock) can't be delayed for a question. The app
menu's Quit is therefore our own item, which asks when projects are running. Every way of
quitting still stops all projects first (`RunEvent::Exit`), recorded as `app_quit`.

### D-038 · "Open in my terminal" runs the Launcher headless, outside its supervision — 2026-10-11
The system terminal runs `repo-launcher --exec <id>` (D-023): the same binary, without Tauri. It
reads the keys from the keychain itself and gives them only to the program it starts; the script
macOS's Terminal needs (`open-in-terminal.command`) holds just the app's path and the project id.
These runs belong to that terminal window (closing it ends them) and aren't shown as running in
the app. This is the one exception to D-004, by design: the user chose their own terminal.

### D-039 · Five starter projects, built in, each really installed in tests — 2026-10-11
An empty library offers five projects that need no keys or account (a Streamlit map, a Gradio
demo, a React and an Astro starter, cowsay), so a first install is one click. The list is compiled
into the engine (`starters.rs`), and a test fails unless each one is installed and launched by
`tests/real_repos.rs` (the manual "Real repos" workflow, 3 OSes). Their names and descriptions
are UI strings. "Try it" adds the project and goes straight to its review: the safety gate is
never skipped, even for our own picks.

### D-040 · Signed updates, checked once a day, installed only when the user says — 2026-10-11
Tauri's updater, with `requireSignedVersion` so an altered `latest.json` can't send people back
to an older signed release. The public key reaches only release builds, through a config overlay
made from a repository variable; other builds have no updater and say so. The check runs at most
once a day, a little after starting, and can be turned off in Settings. It only downloads after
the user clicks "Update and restart", which asks first when projects are running (installing
restarts the app, which stops them, D-005).

### D-041 · Uninstalling: one headless cleanup, used by installers and by Settings — 2026-10-11
`repo-launcher --uninstall [--delete-data]` removes shortcuts, keychain entries and the login
item (the same engine code as `rl remove-shortcuts` / `rl forget-secrets`) and optionally all
data. The Windows uninstallers run it (NSIS hook; an MSI custom action that runs as the user and
not during upgrades). Dragging a Mac app to the Bin runs nothing, and deb/rpm removal runs as
root without the user's keychain, so on those Settings → "Remove all Repo Launcher data" is the
way; it starts the same cleanup with `--after <pid>` and quits so no file is in use. The MSI keeps
user data, as MSI uninstalls do; the setup.exe deletes it when its "Delete the application data"
box is ticked.

### D-042 · UI end-to-end tests run against the mocked engine; no real-app WebDriver yet — 2026-10-11
PLAN §12 planned WebdriverIO + `@wdio/tauri-service` against the real app on 3 OSes per push. That
means building the whole app three times per push, which the Actions budget doesn't allow, and
the engine's real behavior is already covered by the Rust integration tests (fixture projects on
3 OSes) and the shortcut cold-start smoke test. So Playwright runs the UI with the mocked engine
in the Linux CI job: the core flows, keyboard-only use, an axe WCAG 2.1 AA scan in light and dark,
and the pseudo-locale check. D-014 still holds: no build contains a WebDriver endpoint. Revisit if
UI ↔ engine wiring bugs slip through.

### D-043 · A project's own icon, read safely — 2026-10-11
A project's card and shortcuts use its own icon: the file its `launcher.json` names, otherwise a
logo or favicon in the usual places. Only real images (checked by their first bytes), up to
256 KB, never through a symbolic link, and never from outside the project. Shortcuts use it when
it's a square PNG, wrapped as `.icns` / `.ico` without re-encoding, so no image library is
needed; otherwise they keep Repo Launcher's icon. Cards fall back to the owner's avatar, then a
monogram.

### D-044 · Every visible word comes from en.json, checked with a pseudo-locale — 2026-10-11
The Rust shell's own words (tray, macOS app menu including Edit and Window, shortcut tooltips,
`--exec` messages) are read from `ui/src/locales/en.json` too, and a test checks that every key
the shell uses exists. The pseudo-locale (`?lng=pseudo` in the UI, `VITE_PSEUDO=1` for app builds,
`REPO_LAUNCHER_PSEUDO=1` for the shell) accents, brackets and lengthens every string; a
Playwright test fails on any visible text that isn't bracketed. Text that belongs to the user or
a project (names, commands, logs) is marked `translate="no"`, which also tells browser
translators to leave it alone.

### D-045 · Linux packages depend on the AppIndicator library — 2026-10-11
The tray needs `libayatana-appindicator3`; the tray library loads it when the icon is created and
stops the whole app if it's missing, so the deb and rpm declare it. The AppImage bundles it. Desktops that don't show tray icons (GNOME
without the extension) still need the library, but nothing in the app depends on the icon being
visible (D-036).

### D-046 · Older Pythons when a project's pinned packages won't install — 2026-10-19
Many projects pin package versions from before Python 3.12 (`numpy==1.21`, `Pillow==8.4`): those
have ready-made builds only for older Pythons, and on 3.12 they need a compiler or can't be built
at all. When a project doesn't name its Python version (D-016) and installing its packages fails
for such a reason (not when offline or cancelled), the install is tried again on 3.11, 3.10 and
3.9 in turn (only those its `requires-python` allows), each in a fresh environment; the first
failure is the one shown if none works. The fallbacks aren't part of the consent fingerprint
(D-028): they run the same steps on another of our own Pythons.

### D-047 · Python scripts that open a window are desktop apps — 2026-10-19
An entry point that imports tkinter, pygame, Qt, wx, kivy or similar is started like other
desktop apps (no console window), not in the built-in console (D-023). A root `main.py`/`app.py`,
or the file the README says to run, counts as an entry point when it runs code at the top level
even without an `if __name__ == "__main__"` guard: many small projects are written that way.
