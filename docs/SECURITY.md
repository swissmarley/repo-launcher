# Security model

Repo Launcher downloads other people's code and runs it on your computer. This page says
plainly what it protects you from, what it doesn't, and how. If you find a problem, please
report it privately through GitHub's "Report a vulnerability" button on this repository
rather than in a public issue.

## The honest baseline

**Projects run as you, without a sandbox.** A project you install can read and change any file
you can, use your network, and keep running things while it is open. Installing a project
already runs its code: Python's `setup.py`, npm's lifecycle scripts and the project's own setup
steps all execute during **Install**, not only at **Launch**.

So Repo Launcher's job is to:

1. make sure you **know what will happen** before anything runs (the safety gate);
2. give each project **as little as possible** of what isn't its business (your other secrets,
   your environment, your network exposure);
3. **never leak your secrets** (keys and your GitHub sign-in) to disk, logs, other projects or
   the wrong server;
4. keep the app itself and the tools it downloads **trustworthy** (pinned, hashed, signed).

It can't make a malicious project safe. Only install projects from people you trust.

## Threat model

| Threat | In scope? | What we do |
|---|---|---|
| A project you install is malicious | Partly | Safety gate with risk signals before install; it still runs as you if you agree. |
| A project reads your other secrets from the environment | Yes | Children get an allowlisted environment ([`env.rs`](../crates/launcher-core/src/env.rs)): `AWS_*`, `GITHUB_TOKEN`, other projects' keys never reach them. |
| A project reads another project's keys | Yes, through us | Keys are injected only into the project they belong to, only at launch. A malicious project can still read files on disk, but no key is ever on disk (see below). |
| Secrets leak into logs or "Copy diagnostics" | Yes | Every log line and diagnostic is scrubbed (known values + token patterns). Tested. |
| Secrets written to disk | Yes | Keychain only; no plain-text fallback. A test greps every file the engine wrote. |
| Your GitHub token is sent to the wrong server | Yes | Sent only for private repositories, only to GitHub; never for public ones. Tested. |
| Someone on your network uses a project you're running | Yes | Web projects bind to `127.0.0.1` where the framework allows it (D-009). Notebook servers get a fresh random token (D-022). |
| A tampered runtime download (Python, Node, Go, uv) | Yes | SHA-256 hashes compiled into the app (D-015); uv verifies Python builds. |
| A tampered Repo Launcher update | Phase 5 | Signed installers and signed auto-updates. |
| Malicious transitive dependencies (npm/PyPI packages) | No | Not individually reviewed. pnpm projects keep pnpm's default of not running dependencies' install scripts (D-020); npm projects run them, as npm does. |
| Malware already on your computer | No | Out of scope for any desktop app. |
| A project that stays hidden until a later update | Partly | Consent is tied to the exact install/run plan; if an update changes it, you're asked again. Changes to the project's *code* that keep the same plan are not re-reviewed. |

## Secrets

### Where they live

Secrets are your GitHub sign-in, the keys you type for projects (API keys and similar) and,
if you use AI suggestions, your Anthropic key. They are stored **only** in the operating
system's credential store, via [`keyring-core`](https://crates.io/crates/keyring-core):

| OS | Store |
|---|---|
| macOS | the login Keychain |
| Windows | Credential Manager |
| Linux | the Secret Service (GNOME Keyring, KWallet, KeePassXC…) |

All entries use the service name `app.repolauncher.desktop`, with these entry names:
`github.token`, `project.<id>.env.<NAME>`, `ai.anthropic_key`.

**There is no plain-text fallback** (D-007). If Linux has no Secret Service, the keys form and
GitHub sign-in show a clear message instead of saving anything; projects that need no keys
keep working. Projects that read keys from a *file* (for example Streamlit's `secrets.toml`)
are not given one.

One value can be at most **2560 bytes**, Windows Credential Manager's limit. The same limit
applies on every OS so a key that works on one computer works on all of them; longer values
are refused with a clear message.

### The index

The database records the *name* of every keychain entry Repo Launcher creates (never the
value), so removing a project deletes its keys and a full uninstall can delete all of them
(`rl forget-secrets`; the Phase 5 uninstallers call the same code). Repo Launcher never
enumerates your keychain.

Settings a project marks as not secret (`"secret": false` in `launcher.json`, or non-key
settings from a project's `.env.example`) are stored in the database in plain text, and the
form shows them as ordinary text fields.

### When they are used

- **Project keys** are read from the keychain when you click Launch and put into that one
  process's environment. They are not given to install steps.
- **The GitHub token** is sent only when downloading, updating or checking a **private**
  repository, and only to `api.github.com` / `github.com`. Archive downloads for private
  repositories go through GitHub's API, which redirects to a short-lived link; the HTTP client
  drops the `Authorization` header on any redirect to another host. Public repositories never
  receive the token, even when you are signed in (asserted in
  [`tests/credentials.rs`](../crates/launcher-core/tests/credentials.rs)).
- **The Anthropic key** is sent only to `api.anthropic.com`, only when you ask for an AI
  suggestion.

### Scrubbing

Every line written to a project's log file, every line shown in Activity or the console, and
every diagnostic behind "Copy diagnostics" passes through a scrubber
([`redact.rs`](../crates/launcher-core/src/redact.rs)) that replaces:

- the exact value of every secret the engine has handled this session, and
- well-known token formats: GitHub (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`),
  `sk-…` keys (Anthropic, OpenAI and others), Hugging Face, Groq, Replicate, Slack, Google and
  AWS key ids, `Bearer`/`token` headers, `?token=`-style query parameters, passwords in URLs,
  and private-key blocks.

The built-in console hides known values on screen too, as a best effort: if a value arrives
split across two reads of the terminal, the screen may show it, but the log file never does.

The secret type in the engine prints as `[hidden]` in any debug output, and keychain errors
that carry the secret's bytes are never formatted.

### Verified by tests

- `keys_reach_the_project_but_never_the_disk`: a project prints its key; the test checks the
  program received it (its length), that every log and UI event shows `[hidden]`, and that no
  file under the app's data folder (including the SQLite database and its journal) contains
  the value. Removing the project deletes the keychain entry.
- `no_keychain_means_a_clear_message_not_a_file`: with no keychain, saving a key fails with
  `KEYCHAIN_UNAVAILABLE` and nothing is written.
- `private_repos_need_sign_in_and_public_ones_never_send_the_token`: against a mock GitHub.
- Unit tests for the scrubber patterns and the 2560-byte limit.

## GitHub sign-in

Repo Launcher signs in as a **GitHub App** using the **device flow**: you see a short code,
enter it on github.com, and approve. Why this design:

- **No client secret ships in the app.** Desktop apps can't keep a secret; the device flow and
  refreshing device-flow tokens need only the public client ID.
- **Read-only, and only what you choose.** The app asks for *Contents: read* and
  *Metadata: read*, and you pick which repositories it can see. (An OAuth App could reach
  private code only through the `repo` scope, which is read **and write** on every private
  repository.)
- **Short-lived tokens.** Access tokens last 8 hours and are refreshed automatically with a
  6-month refresh token; each refresh replaces both.

**Disconnect** deletes the token from your keychain. Fully revoking Repo Launcher's access
needs a server-side secret it doesn't have, so the app links you to GitHub's
[authorized apps page](https://github.com/settings/apps/authorizations) to finish.

Registering the GitHub App: see [GITHUB_APP.md](GITHUB_APP.md).

## The safety gate

Before a project's **first install**, and again whenever an update changes what will be
installed or run, Repo Launcher downloads the files (nothing runs yet), works out the plan,
and shows:

- **What this will do**, in plain words: download, which runtimes it sets up, what it
  installs and from where, any custom setup steps, which keys it will ask for, and how it
  opens. "Show the exact commands" lists the argv of every step.
- **Serious warnings (red)**, which require typing the project's name to continue:
  - `curl … | sh`-style "download and run" pipelines (also `wget`, PowerShell `iwr | iex`);
  - `sudo`, `doas`, `pkexec`, `runas` and other elevation;
  - obfuscated code: encoded blobs fed to `eval`/`exec`, `marshal.loads`, packed JavaScript;
  - committed program files (ELF, Mach-O, PE, `.exe`/`.so`/…) that the recipe runs;
  - a plan suggested by AI rather than written by the author.
- **Worth knowing (yellow)**: repository less than 30 days old, fewer than 10 stars, owner
  account less than 30 days old, archived, no license, or install-time scripts in the project's
  own manifests (`preinstall`/`postinstall`/`prepare`, `setup.py`).

Pipelines and elevation are looked for in the files the plan actually runs (scripts named in
setup steps or the run command, `package.json` scripts, `setup.py`). Obfuscation is looked for
across the project's own source files. These are **pattern heuristics**: they catch common
dangerous patterns and careless projects, not a determined attacker, who can hide the same
behavior where no pattern looks.

**Consent** is stored per plan fingerprint (`Recipe::fingerprint()`: a SHA-256 of the install
steps, run command, runtimes and web settings; the list of keys a project asks for is left
out, because a new optional key doesn't change what runs). If a later update produces a
different fingerprint, Install/Update stops with `CONSENT_REQUIRED` and the gate is shown again.

**Trusted publishers.** You can tick "Trust projects from *owner*". Projects from trusted
owners skip the gate when there is nothing red; red warnings always show. The list is in
Settings, where owners can be removed.

`launcher.json` commands are validated before anything runs: `sudo`/`su`/`doas`/`runas`/
`pkexec`/`gsudo`, absolute program paths and inline shell code (`bash -c`, `cmd /c`,
`powershell -Command`) are rejected outright (D-008).

## AI-assisted recipes

Off by default. When on, and a project can't be recognized, you can ask Claude (Anthropic) to
suggest a plan, using **your own** Anthropic key from the keychain.

- Before anything is sent, Repo Launcher shows **exactly** what will be sent: the file list,
  the README (shortened) and the project's manifest files. `.env` files, key-like values and
  anything matching a token pattern are never included; text is scrubbed before sending.
- The answer must be a `launcher.json`, which goes through the same validator as an author's
  own file (no elevation, no absolute paths, no inline shell), then through the safety gate,
  where it is always a red warning ("suggested by AI") that needs typed confirmation.
- Nothing suggested by AI ever runs without that confirmation.

## Shortcuts, links and background mode

- **Links.** Repo Launcher registers `repolauncher://`. The only link it acts on is
  `repolauncher://launch/<id>`, and it only ever **launches a project that is already installed
  and approved**: a link can't add, install, update or change anything. Project ids are 8 random
  characters, and browsers ask before opening an app from a link. A link for an unknown id shows
  the main window with "That project isn't in your library anymore."
- **Shortcuts** start the app (`--launch <id>` on Windows and Linux, the link on macOS). They
  contain no keys and no project files.
- **"Open in my terminal"** runs `repo-launcher --exec <id>` in the system's terminal. That
  process reads the project's keys from the keychain at run time and passes them only to the
  program it starts, in memory. Nothing with a key in it is written: on macOS the small
  `open-in-terminal.command` script that Terminal runs contains only the app's path and the id.
- **After a crash**, projects that kept running are recognized by pid, process start time and
  boot time (on POSIX the whole process group), and stopped. A pid that now belongs to another
  program is never signalled.

## Supply chain

- Runtimes (uv, Node, bun, Go) are pinned with SHA-256 hashes compiled into the app; Python
  builds are verified by uv; pnpm and Yarn are checked against the npm registry's `sha512`
  integrity (D-015).
- Rust and JavaScript dependencies are locked (`Cargo.lock`, `pnpm-lock.yaml`).
- The WebView can call only Repo Launcher's own commands: no shell, file system or HTTP access
  from JavaScript, and links open in your browser only if they are `https://`.
- Phase 5 adds signed and notarized installers and signed auto-updates.

## Things that live outside the app's folder

Repo Launcher keeps everything in its own data folder, with these known exceptions:

- **Keychain entries**, listed above, deleted on project removal / full uninstall.
- **Go** writes its telemetry and configuration to the OS user configuration folder
  (`go/telemetry`, `go/env`); Go offers no way to redirect this.
- **Shortcuts** you ask for: `.lnk` files in the Start menu (`Repo Launcher` folder) and on the
  desktop on Windows, a `.desktop` file in `~/.local/share/applications` on Linux, an app in
  `~/Applications` on macOS. Removing the project removes them.
- **Start at login**, if you turn it on: a Launch Agent in `~/Library/LaunchAgents` (macOS), a
  `Run` registry value (Windows) or an autostart `.desktop` file (Linux).
- **The `repolauncher://` link handler**: registered by the installer (and, for development builds
  and AppImages, by the app itself in the per-user registry or applications folder).
- Anything a project itself writes elsewhere: projects run as you.

## Known limits (v1)

- No sandbox; a malicious project can read your files.
- Transitive dependencies' install scripts are not reviewed.
- Risk signals are heuristics.
- Changes to a project's code that keep the same install/run plan don't re-trigger the gate.
- Keys are given to the project's process as environment variables; anything that process
  starts inherits them.
