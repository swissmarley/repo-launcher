# Repo Launcher

Paste a link to a project on GitHub, click **Install**, then **Launch**. Repo Launcher downloads
it, sets up everything it needs in its own private folder, and starts it: web projects open in
your browser. You don't need git, Python, Node or a terminal, and public projects need no account.

> Status: early development (Phase 4 of 5). Python, Node, Go and Docker Compose projects work
> end to end, and projects can describe themselves with a [`launcher.json`](docs/LAUNCHER_JSON.md).
> Private projects open after connecting GitHub, keys live only in the system keychain, and a
> safety review comes before every first install ([docs/SECURITY.md](docs/SECURITY.md)). Projects
> get desktop shortcuts that start them without opening the app's window, the app lives in the
> tray or menu bar, and it can start projects when you log in. See
> [docs/PLAN.md](docs/PLAN.md) for the roadmap and [docs/DECISIONS.md](docs/DECISIONS.md) for
> why things are the way they are.

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
