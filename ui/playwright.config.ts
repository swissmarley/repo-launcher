import { defineConfig, devices } from '@playwright/test'

// End-to-end flows against the UI with the mocked engine (src/lib/mockEngine.ts), in a real
// browser: core flows, keyboard-only use, an axe accessibility scan and the pseudo-locale.
// The real app's engine is covered by the Rust integration tests (DECISIONS D-042).
export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['github']] : 'list',
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 1040, height: 720 } } }],
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
  },
})
