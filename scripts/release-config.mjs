#!/usr/bin/env node
// Prints the Tauri configuration that release builds add to app/tauri.conf.json
// (`tauri build --config <file>`), from the repository's Actions variables. Only public values
// go in it: the update key's public half, the GitHub App's client ID is passed separately, and
// the Azure signing account's names. Secrets stay in environment variables (docs/RELEASING.md).
//
//   UPDATER_PUBKEY           public key from `pnpm tauri signer generate`  → signed auto-updates
//   AZURE_SIGNING_ENDPOINT   e.g. https://weu.codesigning.azure.net         ┐
//   AZURE_SIGNING_ACCOUNT    Artifact Signing account name                  ├ Windows signing
//   AZURE_SIGNING_PROFILE    certificate profile name                       ┘
//
// Without them the release is built unsigned and can't update itself.

const env = (name) => (process.env[name] ?? '').trim()
const config = { bundle: {}, plugins: {} }

const pubkey = env('UPDATER_PUBKEY')
if (pubkey) {
  config.bundle.createUpdaterArtifacts = true
  config.plugins.updater = {
    pubkey,
    endpoints: ['https://github.com/swissmarley/repo-launcher/releases/latest/download/latest.json'],
    // An update's signature must name the version it was released as: an altered latest.json
    // can't send people back to an older, validly signed release.
    requireSignedVersion: true,
    windows: { installMode: 'passive' },
  }
}

const [endpoint, account, profile] = ['AZURE_SIGNING_ENDPOINT', 'AZURE_SIGNING_ACCOUNT', 'AZURE_SIGNING_PROFILE'].map(env)
if (endpoint && account && profile) {
  // trusted-signing-cli reads AZURE_CLIENT_ID, AZURE_CLIENT_SECRET and AZURE_TENANT_ID.
  config.bundle.windows = {
    signCommand: {
      cmd: 'trusted-signing-cli',
      args: ['-e', endpoint, '-a', account, '-c', profile, '-d', 'Repo Launcher', '%1'],
    },
  }
}

process.stdout.write(JSON.stringify(config, null, 2) + '\n')
process.stderr.write(
  `release config: updates ${pubkey ? 'signed' : 'off (no UPDATER_PUBKEY)'}, ` +
    `Windows ${config.bundle.windows ? 'signed' : 'unsigned (no Azure signing variables)'}\n`,
)
