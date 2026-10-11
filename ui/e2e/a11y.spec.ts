import { AxeBuilder } from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'
import { addProject, install, tabTo } from './helpers.ts'

/** WCAG 2.1 A and AA, which includes contrast (PLAN §11). */
async function scan(page: Page, where: string) {
  const { violations } = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze()
  const summary = violations.map((v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(' ')).join(', ')})`)
  expect(summary, `axe on ${where}`).toEqual([])
}

for (const colorScheme of ['light', 'dark'] as const) {
  test(`axe finds nothing on the main screens (${colorScheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' })
    await page.goto('/?update')
    await expect(page.getByText('Or try one of these')).toBeVisible()
    await scan(page, 'the welcome screen')

    await addProject(page, 'github.com/acme/risky-tool')
    const card = page.getByRole('article', { name: 'risky-tool' })
    await card.getByRole('button', { name: 'Install' }).click()
    await expect(page.getByRole('dialog', { name: 'Before you install risky-tool' })).toBeVisible()
    await scan(page, 'the safety review with serious warnings')
    await page.keyboard.press('Escape')

    await addProject(page, 'github.com/acme/chat-app')
    const chat = await install(page, 'chat-app')
    await scan(page, 'the library')
    await chat.getByRole('button', { name: 'Launch' }).click()
    await expect(page.getByRole('dialog', { name: 'chat-app needs a few details' })).toBeVisible()
    await scan(page, 'the keys form')
    await page.keyboard.press('Escape')

    await chat.getByRole('button', { name: 'More actions for chat-app' }).click()
    await expect(page.getByRole('menu')).toBeVisible()
    await scan(page, 'the project menu')
    await page.keyboard.press('Escape')

    await page.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByRole('dialog', { name: 'Settings' })).toBeVisible()
    await scan(page, 'settings')
  })
}

test('keyboard only: add, review, install and launch without a mouse', async ({ page }) => {
  await page.goto('/')
  // The link field has focus on an empty library.
  await page.keyboard.type('github.com/acme/demo')
  await page.keyboard.press('Enter')
  const card = page.getByRole('article', { name: 'demo' })
  await expect(card).toBeVisible()

  await tabTo(page, 'Install')
  await page.keyboard.press('Enter')
  const gate = page.getByRole('dialog', { name: 'Before you install demo' })
  await expect(gate).toBeVisible()
  // Focus moves into the dialog and stays there.
  expect(await gate.evaluate((d) => d.contains(document.activeElement))).toBe(true)
  await tabTo(page, 'Install')
  await page.keyboard.press('Enter')
  await expect(card.getByText('Ready')).toBeVisible({ timeout: 10_000 })

  await tabTo(page, 'Launch')
  await page.keyboard.press('Enter')
  await expect(card.getByRole('button', { name: 'Open' })).toBeVisible()

  // The project menu opens and closes from the keyboard.
  await tabTo(page, 'More actions for demo')
  await page.keyboard.press('Enter')
  await expect(page.getByRole('menu')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('menu')).toBeHidden()
})

test('dialogs keep focus inside and close with Escape', async ({ page }) => {
  await page.goto('/')
  await tabTo(page, 'Settings')
  await page.keyboard.press('Enter')
  const settings = page.getByRole('dialog', { name: 'Settings' })
  await expect(settings).toBeVisible()
  for (let i = 0; i < 25; i++) {
    await page.keyboard.press('Tab')
    expect(await settings.evaluate((d) => d.contains(document.activeElement))).toBe(true)
  }
  await page.keyboard.press('Escape')
  await expect(settings).toBeHidden()
  // Focus returns to what opened it.
  await expect(page.getByRole('button', { name: 'Settings' })).toBeFocused()
})
