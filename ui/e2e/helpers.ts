import { expect, type Page } from '@playwright/test'

/** Adds a project by its link, the way a user pastes one. */
export async function addProject(page: Page, link: string) {
  // By id, so this works in the pseudo-locale too.
  await page.locator('#add-input').fill(link)
  await page.locator('#add-input').press('Enter')
}

/** Install from a card: the safety review opens; agree and wait until it's ready. */
export async function install(page: Page, name: string) {
  const card = page.getByRole('article', { name })
  await card.getByRole('button', { name: 'Install' }).click()
  const gate = page.getByRole('dialog', { name: `Before you install ${name}` })
  await gate.getByRole('button', { name: 'Install' }).click()
  await expect(card.getByText('Ready')).toBeVisible({ timeout: 10_000 })
  return card
}

/**
 * Presses Tab until the focused element has this accessible name (keyboard-only use), and
 * checks that focus is visible there.
 */
export async function tabTo(page: Page, name: string | RegExp, max = 40) {
  for (let i = 0; i < max; i++) {
    await page.keyboard.press('Tab')
    const focused = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null
      if (!el) return null
      const label = el.getAttribute('aria-label') ?? (el.textContent ?? '').trim()
      return { label, outline: getComputedStyle(el).outlineStyle }
    })
    if (focused && (typeof name === 'string' ? focused.label === name : name.test(focused.label))) {
      expect(focused.outline, `focus is visible on “${focused.label}”`).not.toBe('none')
      return
    }
  }
  throw new Error(`Tab never reached “${name}”`)
}
