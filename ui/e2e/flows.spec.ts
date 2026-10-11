import { expect, test } from '@playwright/test'
import { addProject, install } from './helpers.ts'

test('first run: a starter project goes straight to its review, then installs and launches', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Run projects from GitHub — no setup needed' })).toBeVisible()
  const starters = page.getByRole('list', { name: 'Or try one of these' }).getByRole('listitem')
  await expect(starters).toHaveCount(5)
  // The link field is ready to paste into.
  await expect(page.getByLabel('Link to a GitHub project')).toBeFocused()

  await page.getByRole('button', { name: 'Try Uber pickups in New York' }).click()
  const gate = page.getByRole('dialog', { name: /^Before you install/ })
  await expect(gate.getByText('What this will do')).toBeVisible()
  await gate.getByRole('button', { name: 'Install' }).click()

  const card = page.getByRole('article', { name: 'demo-uber-nyc-pickups' })
  await expect(card.getByText('Ready')).toBeVisible({ timeout: 10_000 })
  await card.getByRole('button', { name: 'Launch' }).click()
  await expect(card.getByRole('button', { name: 'Open' })).toBeVisible()
  await card.getByRole('button', { name: 'Stop' }).click()
  await expect(card.getByRole('button', { name: 'Launch' })).toBeVisible()
})

test('a mistyped or private link explains itself', async ({ page }) => {
  await page.goto('/')
  await addProject(page, 'not a link')
  await expect(page.getByRole('alert')).toContainText("That doesn't look like a GitHub link.")
  await addProject(page, 'github.com/acme/missing')
  const alert = page.getByRole('alert')
  await expect(alert).toContainText("We couldn't find acme/missing.")
  await expect(alert.getByRole('button', { name: 'Connect GitHub' })).toBeVisible()
})

test('serious warnings need the project name typed before installing', async ({ page }) => {
  await page.goto('/')
  await addProject(page, 'github.com/acme/risky-tool')
  const card = page.getByRole('article', { name: 'risky-tool' })
  await card.getByRole('button', { name: 'Install' }).click()
  const gate = page.getByRole('dialog', { name: 'Before you install risky-tool' })
  await expect(gate.getByText('Serious warnings')).toBeVisible()
  const go = gate.getByRole('button', { name: 'Install anyway' })
  await expect(go).toBeDisabled()
  await gate.getByLabel('To install anyway, type risky-tool').fill('risky-tool')
  await expect(go).toBeEnabled()
})

test('a project that needs a key asks for it before its first launch', async ({ page }) => {
  await page.goto('/')
  await addProject(page, 'github.com/acme/chat-app')
  const card = await install(page, 'chat-app')
  await card.getByRole('button', { name: 'Launch' }).click()
  const form = page.getByRole('dialog', { name: 'chat-app needs a few details' })
  await expect(form.getByLabel(/OpenAI API key/)).toBeVisible()
  await form.getByLabel(/OpenAI API key/).fill('sk-test-0000')
  await form.getByRole('button', { name: 'Save and launch' }).click()
  await expect(card.getByRole('button', { name: 'Open' })).toBeVisible()
})

test('removing the last project brings the welcome back', async ({ page }) => {
  await page.goto('/')
  await addProject(page, 'github.com/acme/demo')
  const card = page.getByRole('article', { name: 'demo' })
  await card.getByRole('button', { name: 'More actions for demo' }).click()
  await page.getByRole('menuitem', { name: 'Remove' }).click()
  await page.getByRole('alertdialog').getByRole('button', { name: 'Remove' }).click()
  await expect(page.getByRole('heading', { name: 'Run projects from GitHub — no setup needed' })).toBeVisible()
})

test('updates: checking by hand, and the banner when a new version is ready', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'Settings' }).click()
  const settings = page.getByRole('dialog', { name: 'Settings' })
  await expect(settings.getByText('You have version 0.1.0.')).toBeVisible()
  await settings.getByRole('button', { name: 'Check for updates' }).click()
  await expect(settings.getByText('You have the latest version.')).toBeVisible()

  await page.goto('/?update')
  const banner = page.getByRole('status').filter({ hasText: 'Repo Launcher 0.2.0 is ready to install.' })
  await expect(banner).toBeVisible()
  await banner.getByRole('button', { name: 'Later' }).click()
  await expect(banner).toBeHidden()
})

test('"Remove all data" says what it deletes before doing anything', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('button', { name: 'Remove all data…' }).click()
  const confirm = page.getByRole('alertdialog', { name: 'Remove everything Repo Launcher set up?' })
  await expect(confirm.getByText(/keys you entered/)).toBeVisible()
  await confirm.getByRole('button', { name: 'Cancel' }).click()
  await expect(confirm).toBeHidden()
})
