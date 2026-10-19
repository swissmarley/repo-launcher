# Releasing Repo Launcher

How installers are built, signed and published, and exactly what has to be set up before a
release can be signed. Until those accounts exist, everything below works **unsigned**: the
workflow builds every installer, and the app simply can't update itself.

## What the release workflow does

[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs only when started by
hand or for a version tag. It never runs on ordinary pushes (the Actions budget is small).

| Runner | Builds |
|---|---|
| `macos-latest` | `Repo Launcher_<version>_universal.dmg` (Apple silicon and Intel in one app) |
| `windows-latest` | `Repo Launcher_<version>_x64_en-US.msi` and `Repo Launcher_<version>_x64-setup.exe` |
| `ubuntu-22.04` | `.AppImage`, `.deb`, `.rpm` (built on the oldest supported Ubuntu, for glibc) |

- **Started by hand** (Actions → Release → Run workflow, or
  `gh workflow run release.yml --ref main`): builds everything and keeps it as workflow
  artifacts. No release is created. Use this to try installers on clean machines.
- **A tag `v<version>`**: also uploads the installers, their update signatures and
  `latest.json` to a **draft** GitHub release. Nothing reaches users until you publish the draft
  (the updater only sees published releases).

Every installer registers the `repolauncher://` link scheme (macOS `Info.plist`, the Windows
installers' registry entries, the Linux `.desktop` file), and both Windows installers run the
uninstall cleanup described in [Uninstalling](#uninstalling).

## Making a release

1. Pick the version (semver) and set it in `app/tauri.conf.json` (`version`, which the installers
   and the updater use) and `Cargo.toml` (`[workspace.package] version`). The workflow refuses a
   tag that doesn't match `app/tauri.conf.json`.
2. Make sure `main` is green, including one manual 3-OS run:
   `gh workflow run ci.yml --ref main`.
3. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
4. When the workflow finishes, open the draft release. Check that it has the dmg, msi, setup.exe,
   AppImage, deb and rpm, plus `latest.json` and `.sig` files (only if updates are set up).
5. Run the clean-VM checks in [QA.md](QA.md) with these exact files.
6. Publish the draft. Running copies of Repo Launcher find it within a day (or at once with
   Settings → Updates → Check for updates).

## Setting up signing and signed updates

Each part switches on by itself once its secrets and variables exist in the repository
(Settings → Secrets and variables → Actions). Variables hold public values; secrets hold
everything else. None of them is ever written into a file, an installer or update metadata.

### 1. Signed auto-updates (free; do this first)

Updates are checked against a key pair. The public half is compiled into release builds; the
private half signs each update. **If the private key is lost, existing installations can never
be updated again**, and if it leaks, someone else can sign updates. Keep it in a password manager.

```bash
pnpm tauri signer generate -w ~/.tauri/repo-launcher.key
```

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/repo-launcher.key
```

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

```bash
gh variable set UPDATER_PUBKEY < ~/.tauri/repo-launcher.key.pub
```

Then move `~/.tauri/repo-launcher.key` into your password manager and delete the file.

| Name | Kind | What |
|---|---|---|
| `UPDATER_PUBKEY` | variable | The public key (contents of the `.pub` file) |
| `TAURI_SIGNING_PRIVATE_KEY` | secret | The private key (contents of the key file) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | secret | Its password |

With `UPDATER_PUBKEY` set, `scripts/release-config.mjs` adds the updater configuration to release
builds: the endpoint `https://github.com/swissmarley/repo-launcher/releases/latest/download/latest.json`,
the public key, and `requireSignedVersion` (an update's signature must name the version it was
released as, so an altered `latest.json` can't push people back to an older, validly signed
release). Builds without it (development builds, anyone's own build) say in Settings that they
can't update themselves.

### 2. macOS: Developer ID signing and notarization

Without this, macOS blocks the downloaded app until the user allows it in System Settings →
Privacy & Security (**Open Anyway**). Until then, release builds are signed ad hoc
(`scripts/release-config.mjs`): on Apple silicon an app with no signature at all is reported as
"damaged" and can't be opened that way. The release notes tell users how to open unsigned builds.

Needed: an **Apple Developer Program** membership (paid, yearly) for the account that will sign.

1. In Xcode → Settings → Accounts, or on developer.apple.com → Certificates, create a
   **Developer ID Application** certificate.
2. In Keychain Access, export it with its private key as a `.p12` file with a password.
3. On appleid.apple.com → Sign-In and Security → App-Specific Passwords, create one for
   notarization.
4. Store them (the certificate goes in base64, straight from the file into the secret):

```bash
base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE
```

| Name | Kind | What |
|---|---|---|
| `APPLE_CERTIFICATE` | secret | The `.p12`, base64 |
| `APPLE_CERTIFICATE_PASSWORD` | secret | The `.p12`'s password |
| `APPLE_SIGNING_IDENTITY` | secret | e.g. `Developer ID Application: Your Name (TEAMID1234)` |
| `APPLE_ID` | secret | The Apple ID's email |
| `APPLE_PASSWORD` | secret | The app-specific password from step 3 |
| `APPLE_TEAM_ID` | secret | The 10-character team ID |

Then delete the `.p12` file. When `APPLE_CERTIFICATE` exists, the macOS job uses the signed
build step; Tauri imports the certificate into a temporary keychain, signs with the hardened
runtime, notarizes with Apple and staples the ticket to the dmg.

Note for users of earlier unsigned builds: the system keychain ties saved keys to the app's
signature, so the first signed version may ask once to allow access to them.

### 3. Windows: Azure Artifact Signing

Without this, Windows SmartScreen warns "Windows protected your PC" on download.

Needed: an Azure subscription with an **Artifact Signing** (formerly Trusted Signing) account.
Check Microsoft's current eligibility rules for your country and whether you sign as an
individual or an organization; identity validation can take several days.

1. Create the Artifact Signing account and a **certificate profile** (Public Trust), and complete
   identity validation.
2. Create an app registration (service principal) and give it the
   *Artifact Signing Certificate Profile Signer* role on the account. Create a client secret.

| Name | Kind | What |
|---|---|---|
| `AZURE_SIGNING_ENDPOINT` | variable | The account's region endpoint, e.g. `https://weu.codesigning.azure.net` |
| `AZURE_SIGNING_ACCOUNT` | variable | The account name |
| `AZURE_SIGNING_PROFILE` | variable | The certificate profile name |
| `AZURE_TENANT_ID` | secret | The directory (tenant) ID |
| `AZURE_CLIENT_ID` | secret | The app registration's client ID |
| `AZURE_CLIENT_SECRET` | secret | Its client secret |

With the three variables set, the Windows job installs `trusted-signing-cli` and Tauri signs the
program, the msi and the setup.exe with it (`bundle.windows.signCommand`). Any other signing
service works the same way: change the command in `scripts/release-config.mjs`.

### 4. GitHub sign-in in release builds

Public projects need nothing. For "Connect GitHub" in release builds, register the GitHub App
([GITHUB_APP.md](GITHUB_APP.md)) and set two public variables:

| Name | Kind | What |
|---|---|---|
| `GITHUB_APP_CLIENT_ID` | variable | The app's client ID (public; no client secret exists) |
| `GITHUB_APP_SLUG` | variable | The app's URL name, for "Choose repositories" |

### Linux

The AppImage, deb and rpm are not signed. Distribution-level signing (a GPG-signed apt or dnf
repository) is a later step if Repo Launcher gets its own package repository.

## Uninstalling

What an uninstaller can't do by deleting the program's files is in one place: the program's
headless mode `repo-launcher --uninstall [--delete-data]`. It removes the shortcuts Repo
Launcher made, its keychain entries (the same engine code as `rl remove-shortcuts` and
`rl forget-secrets`) and its start-at-login entry; with `--delete-data` also its projects,
downloaded tools, logs and WebView data.

| How it was installed | How it's removed | What runs |
|---|---|---|
| Windows setup.exe | Settings → Apps → Installed apps | `--uninstall` before the files go; with "Delete the application data" ticked, `--delete-data` after |
| Windows msi | Settings → Apps → Installed apps | `--uninstall` before the files go (as the user who uninstalls). Projects stay in the user's folder, as MSI uninstalls leave user data |
| macOS dmg | Drag the app to the Bin | Nothing can run then: first use Settings → Remove all data |
| Linux deb/rpm | The package manager | Runs as root, so it can't reach the user's keychain: first use Settings → Remove all data |
| AppImage | Delete the file | First use Settings → Remove all data |

**Settings → Remove all Repo Launcher data** does the full cleanup on every OS: it stops all
projects, removes shortcuts, keychain entries, the start-at-login entry and the link
registration, quits, and a copy of itself (`--uninstall --delete-data --after <pid>`) deletes
the files once the app has exited.

Updates never run any of this: the Windows updater runs the installer in update mode
(`/UPDATE`), and MSI upgrades are excluded with `NOT UPGRADINGPRODUCTCODE`.

## What is never in an installer or update

- No keys, tokens or passwords of anyone's: users' keys live only in their system keychain.
- No GitHub client secret (none exists; the device flow needs only the public client ID).
- Signing keys exist only as environment variables of the signing step; GitHub masks secrets in
  logs, and the workflow never prints or writes them.
- `latest.json` holds versions, download links and signatures only.
