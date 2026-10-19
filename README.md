# Repo Launcher

Paste a link to a project on GitHub, click **Install**, then **Launch**. Repo Launcher downloads
it, sets up everything it needs in its own private folder, and starts it: web projects open in
your browser. You don't need git, Python, Node or a terminal, and public projects need no account.

> Status: Phase 5 of 5 (polish and ship). Python, Node, Go and Docker Compose projects and plain
> websites work end to end, and projects can describe themselves with a
> [`launcher.json`](docs/LAUNCHER_JSON.md).
> Private projects open after connecting GitHub, keys live only in the system keychain, and a
> safety review comes before every first install ([docs/SECURITY.md](docs/SECURITY.md)). Projects
> get desktop shortcuts, the app lives in the tray or menu bar, and it can start projects when you
> log in. Installers are built for macOS, Windows and Linux ([docs/RELEASING.md](docs/RELEASING.md));
> they stay unsigned until signing accounts exist. See [docs/PLAN.md](docs/PLAN.md) for the
> roadmap, [docs/DECISIONS.md](docs/DECISIONS.md) for why things are the way they are, and
> [docs/QA.md](docs/QA.md) for the manual checks before a release.

## Removing Repo Launcher

Settings → **Remove all Repo Launcher data** stops every project and removes everything it set
up: projects, the keys you entered (from the system keychain), shortcuts and starting at login.
Then delete the app as usual. On Windows, uninstalling from Settings → Apps does the same.

## Developing

Requirements: Rust (stable), Node 22+, pnpm 10.

```bash
pnpm install
```

```bash
pnpm tauri dev
```

The UI alone (with a mock engine, in any browser):

```bash
pnpm --filter ui dev
```

The engine from the command line (`RL_HOME` keeps test data separate from the app's own):

```bash
RL_HOME=/tmp/rl cargo run -p rl -- add https://github.com/streamlit/demo-uber-nyc-pickups
```

### Tests

```bash
cargo test -p launcher-core -p rl
```

```bash
cargo test -p launcher-core --test python_projects -- --ignored --test-threads=1
```

```bash
pnpm --filter ui test
```

The UI's flows, keyboard use, an accessibility scan and the pseudo-locale check, in a real
browser against the mock engine (run `pnpm --filter ui exec playwright install chromium` once):

```bash
pnpm --filter ui e2e
```

Try the UI in the pseudo-locale, which shows any text that doesn't come from `en.json`:
open `http://localhost:1420/?lng=pseudo` while `pnpm --filter ui dev` runs.

Licenses and advisories of Rust dependencies:

```bash
cargo deny check
```

After changing a type the UI uses, regenerate the TypeScript bindings:

```bash
cargo test -p launcher-core --features ts --lib
```

## Layout

| Path | What |
|---|---|
| `crates/launcher-core` | The engine: detection, recipes, runtimes, install, run. No UI dependency. |
| `crates/launcher-cli` | `rl`, a small CLI over the engine for development and tests. |
| `app` | The Tauri desktop shell: thin command adapters, no business logic. |
| `ui` | The React interface. |
| `fixtures/synthetic` | Tiny projects used by integration tests. |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Third-party components and their licenses are listed
in [docs/THIRD_PARTY.md](docs/THIRD_PARTY.md).
