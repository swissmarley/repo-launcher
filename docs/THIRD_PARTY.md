# Third-party software

Repo Launcher is MIT OR Apache-2.0. This page lists what it is built from and what it downloads,
and how the licenses are checked.

## How it's checked

- **Rust**: `cargo deny check` runs in CI on every push ([`deny.toml`](../deny.toml)). It fails on
  any license not in the allow list, on known security advisories, on yanked crates, and on
  crates from anywhere but crates.io. It checks the dependency tree of all shipped targets
  (Linux x64, macOS arm64 and x64, Windows x64).
- **JavaScript**: the UI's production dependencies are listed with
  `pnpm --filter ui licenses list --prod`. Everything is bundled into the app's own JavaScript
  by Vite; dev-only tools (Vite, Vitest, Playwright, oxlint, Tailwind's compiler) are not shipped.

Allowed licenses: MIT, MIT-0, Apache-2.0 (also WITH LLVM-exception), BSD-2-Clause,
BSD-3-Clause, BSL-1.0, CC0-1.0, ISC, MPL-2.0, Unicode-3.0, Unlicense, Zlib, 0BSD, bzip2-1.0.6.
There are no GPL, AGPL or LGPL-only dependencies. Crates offered under several licenses
(for example `MIT OR Apache-2.0 OR LGPL-2.1-or-later`) are used under a permissive one.

## Rust crates in the app (checked 2026-10-11, 511 crates)

| Licenses involved | Crates | Notes |
|---|---|---|
| MIT and/or Apache-2.0 | ~430 | Tauri, tokio, serde, reqwest, rusqlite, keyring-core, portable-pty, … |
| Unicode-3.0 | 18 | ICU4X crates used by URL handling (`idna`) |
| Zlib (or a choice including it) | ~15 | `miniz_oxide`, `zlib-rs`, `foldhash`, objc2 bindings |
| Unlicense OR MIT | 7 | `memchr`, `aho-corasick`, `walkdir`, … |
| BSD-3-Clause (or a choice) | ~10 | `brotli`, `subtle`, `zstd`, `encoding_rs`, `moxcms` |
| **MPL-2.0** | 5 | `cssparser`, `cssparser-macros`, `dtoa-short`, `selectors` (Linux WebView HTML tooling), `option-ext` (via `dirs`). File-level copyleft: we ship them unmodified and their sources are on crates.io. |
| ISC (or a choice) | ~7 | `rustls`, `rustls-webpki`, `ring`, `untrusted`, `libloading` |
| CC0-1.0 / MIT-0 | 4 | `dunce`, `constant_time_eq`, `tiny-keccak`, `ppmd-rust` |
| BSL-1.0 OR Apache-2.0 | 1 | `ryu` |
| BSD-2-Clause OR Apache-2.0 | 1 | `serial2` |
| bzip2-1.0.6 | 1 | `libbz2-rs-sys` (zip archives) |
| 0BSD OR MIT OR Apache-2.0 | 1 | `adler2` |

Regenerate the full list with `cargo deny list -l crate`. SQLite (bundled by `rusqlite`) is in
the public domain.

One advisory is ignored, with its reason in `deny.toml`: RUSTSEC-2024-0370 (`proc-macro-error`
is unmaintained), a build-time macro helper inside gtk-rs 0.18, which Tauri 2's Linux WebView
and tray use. It is not a vulnerability.

## JavaScript in the UI

| License | Packages |
|---|---|
| MIT | React, React DOM, Radix UI primitives, i18next, react-i18next, xterm.js and its fit addon, Floating UI, and their small helpers (86 packages in all) |
| Apache-2.0 OR MIT | `@tauri-apps/api` |
| 0BSD | `tslib` |

Fonts are the operating system's own; no font files are shipped. The app icon and the menu bar
icon (`app/icons/source.svg`, `app/icons/tray-template.svg`) are original to this project.

## The operating system's WebView

Repo Launcher uses the WebView the system provides and doesn't ship one: WebKit (macOS),
Microsoft Edge WebView2 (Windows; the Windows installers fetch Microsoft's installer if it's
missing) and WebKitGTK (Linux, a package dependency).

## Tools downloaded when a project needs them

These aren't part of Repo Launcher's installers. When a project needs them, Repo Launcher
downloads the official builds into its own folder and checks them against hashes compiled into
the app (D-015). Each comes with its own license files.

| Tool | Version pinned | License |
|---|---|---|
| uv | 0.13.0 | MIT OR Apache-2.0 |
| Python (builds installed by uv, from python-build-standalone) | per project, default 3.12 | PSF License, plus the licenses of the libraries each build includes |
| Node.js | 24.21.0, 22.23.3 | MIT, plus bundled third-party licenses |
| pnpm | from the npm registry (default 10) | MIT |
| Yarn | Classic 1.22.22, Berry 4 | BSD-2-Clause |
| Bun | 1.4.3 | MIT (it includes JavaScriptCore, LGPL-2.1, as Bun distributes it) |
| Go | 1.27.2 | BSD-3-Clause |

Projects themselves, and the packages they install from PyPI or npm, keep their own licenses;
Repo Launcher shows a project's license on its card and a yellow warning when it has none.
