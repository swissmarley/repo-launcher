# Registering the GitHub App

Repo Launcher signs users in to GitHub with a **GitHub App** and the **device flow**. Only
private projects and the "Your GitHub projects" list need it; public projects never do.
The app's **client ID** is public and is the only value Repo Launcher needs. No client secret
is created for, or shipped with, the desktop app.

## 1. Create the app

1. Open <https://github.com/settings/apps/new> (or, for an organization,
   *Organization settings → Developer settings → GitHub Apps → New GitHub App*).
2. **GitHub App name**: `Repo Launcher` (must be unique on GitHub; add a suffix if taken).
   The name becomes the app's *slug*, e.g. `repo-launcher`.
3. **Homepage URL**: `https://github.com/swissmarley/repo-launcher`.
4. **Callback URL**: leave empty. The device flow doesn't use one.
5. **Expire user authorization tokens**: keep **checked**. Tokens then last 8 hours and are
   refreshed automatically.
6. **Request user authorization (OAuth) during installation**: leave unchecked.
7. **Enable Device Flow**: **check this**. Without it sign-in fails with
   `device_flow_disabled`.
8. **Webhook**: uncheck **Active**. Repo Launcher has no server.
9. **Permissions → Repository permissions**:
   - **Contents: Read-only**
   - **Metadata: Read-only** (selected automatically)
   - leave everything else at *No access*.
   No organization or account permissions.
10. **Where can this GitHub App be installed?**: **Any account**, so every user can install
    it on their own repositories.
11. Click **Create GitHub App**.

Don't generate a client secret or a private key: Repo Launcher uses neither.

## 2. Give Repo Launcher the client ID

On the app's settings page, copy the **Client ID** (it starts with `Iv`; it is *not* the
numeric App ID). Repo Launcher reads it, in this order:

1. the `REPO_LAUNCHER_GITHUB_CLIENT_ID` environment variable when the app starts;
2. the app's settings database, set with the CLI:
   ```bash
   cargo run -p rl -- config github-app Iv23xxxxxxxxxxxxxx repo-launcher
   ```
3. a value compiled into release builds: set `REPO_LAUNCHER_GITHUB_CLIENT_ID` (and
   `REPO_LAUNCHER_GITHUB_APP_SLUG`) in the build environment, e.g. as GitHub Actions
   variables in the release workflow (Phase 5).

The optional **app slug** (`REPO_LAUNCHER_GITHUB_APP_SLUG`, or the second CLI argument) lets
Repo Launcher link straight to `https://github.com/apps/<slug>/installations/new`, where users
choose which repositories it may read. Without it the link goes to
`https://github.com/settings/installations`.

## 3. Try it

```bash
cargo run -p rl -- github login
```

```bash
cargo run -p rl -- github repos
```

Then, in the app, **Connect GitHub**, add a private repository you've shared with the app,
and install it.

## How users experience it

1. **Connect GitHub** shows an 8-character code and a "Copy code and open GitHub" button.
2. On github.com the user enters the code and approves *Repo Launcher (read-only)*.
3. The first time, GitHub offers to **install** the app; the user picks repositories
   (all, or selected ones). "Choose which projects Repo Launcher can see" in Settings
   reopens that page later.
4. The access token is stored in the OS keychain only. **Disconnect** deletes it and links to
   <https://github.com/settings/apps/authorizations> to revoke access completely.

## Why a GitHub App and not an OAuth App

An OAuth App can reach private code only through the `repo` scope, which grants read **and
write** on every private repository the user can access, with tokens that never expire. A
GitHub App's user token is limited to the permissions above (read-only), to the repositories
the user chose, and expires after 8 hours. See [SECURITY.md](SECURITY.md#github-sign-in).
