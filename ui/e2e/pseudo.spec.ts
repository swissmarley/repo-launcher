import { expect, test, type Page } from '@playwright/test'
import { addProject } from './helpers.ts'

/**
 * In the pseudo-locale every string from en.json is wrapped in ⟦…⟧. Any visible text that
 * isn't, outside elements marked translate="no" (project names, commands, logs), was
 * hard-coded instead of coming from en.json.
 */
async function hardCoded(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = []
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
    for (let n = walker.nextNode(); n; n = walker.nextNode()) {
      const el = n.parentElement
      const text = (n.textContent ?? '').trim()
      if (!el || !/[A-Za-z]/.test(text)) continue
      if (el.closest('[translate="no"], script, style, [hidden]')) continue
      if (!el.checkVisibility()) continue
      // Inside a pseudo string (interpolated values sit between the brackets).
      const whole = el.textContent ?? ''
      if (whole.includes('⟦') && whole.includes('⟧')) continue
      out.push(text)
    }
    for (const el of document.querySelectorAll<HTMLElement>('[placeholder], [aria-label], [title]')) {
      if (el.closest('[translate="no"]')) continue
      for (const attr of ['placeholder', 'aria-label', 'title']) {
        const v = el.getAttribute(attr)
        if (v && /[A-Za-z]/.test(v) && !v.includes('⟦')) out.push(`${attr}="${v}"`)
      }
    }
    return out
  })
}

test('every visible word comes from en.json (pseudo-locale)', async ({ page }) => {
  await page.goto('/?lng=pseudo&update')
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-XA')
  await expect(page.getByRole('heading', { level: 2 })).toContainText('⟦')
  expect(await hardCoded(page), 'welcome').toEqual([])

  await addProject(page, 'github.com/acme/risky-tool')
  const card = page.getByRole('article', { name: 'risky-tool' })
  await expect(card).toBeVisible()
  expect(await hardCoded(page), 'library').toEqual([])

  await card.getByRole('button').first().click()
  await expect(page.getByRole('dialog')).toBeVisible()
  expect(await hardCoded(page), 'safety review').toEqual([])
  await page.keyboard.press('Escape')

  await page.getByRole('button', { name: /^⟦Š/ }).click()
  await expect(page.getByRole('dialog')).toBeVisible()
  expect(await hardCoded(page), 'settings').toEqual([])
})
