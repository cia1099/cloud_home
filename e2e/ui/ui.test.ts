import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import type { Browser, BrowserContext, Page } from 'playwright-core'
import { LOCALES, LOCALE_NAMES, translate, type Locale } from '../../ui/src/i18n/locale'
import type { MessageKey } from '../../ui/src/i18n/messages/en'
import { ADMIN_PASSWORD, launch, startServer, USER_PASSWORD, type Server } from './harness'
import { shot } from './media'

let server: Server
let browser: Browser

beforeAll(async () => {
  server = await startServer()
  browser = await launch()
})
afterAll(async () => {
  await browser?.close()
  server?.stop()
})

async function page(locale: string, scheme: 'light' | 'dark' = 'light'): Promise<{ ctx: BrowserContext; page: Page }> {
  const ctx = await browser.newContext({ locale, colorScheme: scheme, viewport: { width: 1280, height: 800 } })
  ctx.setDefaultTimeout(10_000)
  return { ctx, page: await ctx.newPage() }
}

const t = (locale: Locale, key: MessageKey, params?: Record<string, string | number>) => translate(locale, key, params)

describe('browser-language auto-detection (nothing stored)', () => {
  const cases: [string, Locale][] = [
    ['zh-TW', 'zh-Hant'],
    ['zh-HK', 'zh-Hant'],
    ['zh-Hant', 'zh-Hant'],
    ['zh-CN', 'zh-Hans'],
    ['zh-SG', 'zh-Hans'],
    ['zh', 'zh-Hans'],
    ['en-US', 'en'],
    ['en-GB', 'en'],
    ['fr-FR', 'en'],
    ['de-DE', 'en'],
    ['ja-JP', 'en'],
  ]
  test.each(cases)('browser language %s shows the %s UI', async (browserLang, expected) => {
    const { ctx, page: p } = await page(browserLang)
    await p.goto(server.url + '/signin')
    await p.getByRole('heading', { name: t(expected, 'auth.sign_in.title') }).waitFor()
    expect(await p.locator('html').getAttribute('lang')).toBe(expected)
    await ctx.close()
  })
})

describe('language and appearance choices', () => {
  test('the language switch on the sign-in page overrides detection and survives a reload', async () => {
    const { ctx, page: p } = await page('zh-TW')
    await p.goto(server.url + '/signin')
    await p.getByRole('button', { name: LOCALE_NAMES.en, exact: true }).click()
    await p.getByRole('heading', { name: t('en', 'auth.sign_in.title') }).waitFor()
    await p.reload()
    await p.getByRole('heading', { name: t('en', 'auth.sign_in.title') }).waitFor()
    expect(await p.locator('html').getAttribute('lang')).toBe('en')
    await ctx.close()
  })

  test.each(LOCALES)('Settings switches appearance and language (%s)', async (locale) => {
    const { ctx, page: p } = await page('en-US', 'light')
    await p.goto(server.url + '/register')
    await p.getByLabel('Display name').fill(`Prefs ${locale}`)
    await p.getByLabel('Email', { exact: true }).fill(`prefs-${locale}@example.test`)
    await p.getByLabel('Password', { exact: true }).fill(USER_PASSWORD)
    await p.getByRole('button', { name: 'Create Account', exact: true }).click()
    await p.getByText(t('en', 'files.empty.title')).waitFor()
    await p.goto(server.url + '/settings')
    await p.getByRole('button', { name: LOCALE_NAMES[locale], exact: true }).click()
    await p.getByRole('heading', { name: t(locale, 'settings.title') }).waitFor()

    const theme = () => p.locator('html').getAttribute('data-theme')
    expect(await theme()).toBe('light') // Automatic follows the system (light)
    await p.getByRole('button', { name: t(locale, 'settings.appearance.dark'), exact: true }).click()
    expect(await theme()).toBe('dark')
    await p.reload()
    expect(await theme()).toBe('dark') // remembered
    await p.getByRole('button', { name: t(locale, 'settings.appearance.light'), exact: true }).click()
    expect(await theme()).toBe('light')
    await p.getByRole('button', { name: t(locale, 'settings.appearance.auto'), exact: true }).click()
    await p.emulateMedia({ colorScheme: 'dark' })
    await p.waitForFunction(() => document.documentElement.dataset.theme === 'dark') // Automatic follows a system change live
    expect(await theme()).toBe('dark')
    await ctx.close()
  })
})

describe('admin update page with an update available (API answers are stubbed)', () => {
  // A dev run cannot update itself, so the real server only shows the "not available" state
  // (covered by the journey). Here the admin endpoint is stubbed to show the other states.
  test.each(LOCALES)('shows version, offers Install and follows the install (%s)', async (locale) => {
    const { ctx, page: p } = await page(locale === 'en' ? 'en-US' : locale === 'zh-Hans' ? 'zh-CN' : 'zh-TW', 'dark')
    const status = {
      enabled: true,
      phase: 'idle',
      current_version: '0.3.1',
      available: { version: '0.4.0', notes_url: 'https://example.test/notes', size: 12_345_678 },
      last_checked_at: new Date().toISOString(),
      last_error: null,
      settings: { mode: 'manual', window: '03:00-05:00', channel: 'stable', check_interval_hours: 6 },
    }
    const installs: unknown[] = []
    await p.route('**/api/v1/admin/update**', async (route) => {
      const req = route.request()
      if (req.method() === 'POST' && req.url().endsWith('/install')) {
        installs.push(req.postDataJSON())
        status.phase = 'downloading'
      }
      if (req.method() === 'PUT') Object.assign(status.settings, req.postDataJSON())
      await route.fulfill({ json: { data: status } })
    })

    await p.goto(server.url + '/register')
    await p.getByRole('textbox').first().fill(`Admin ${locale}`)
    await p.locator('input[type=email]').fill(`admin-${locale}@example.test`)
    await p.locator('input[type=password]').fill(USER_PASSWORD)
    await p.locator('form button[type=submit]').click()
    await p.getByText(t(locale, 'files.empty.title')).waitFor()
    await p.goto(server.url + '/admin')
    await p.getByLabel(t(locale, 'admin.password')).fill(ADMIN_PASSWORD)
    await p.getByRole('button', { name: t(locale, 'admin.continue') }).click()
    await p.getByText(`${t(locale, 'admin.update_available')}: v0.4.0`).waitFor()
    await p.getByText(`${t(locale, 'admin.current_version')} v0.3.1`).waitFor()
    await shot(p, `update-available-${locale}`)

    await p.getByRole('button', { name: t(locale, 'admin.automatic'), exact: true }).click()
    await p.getByText(t(locale, 'admin.saved')).waitFor()
    await p.getByRole('button', { name: t(locale, 'admin.install'), exact: true }).first().click()
    await p.getByText(t(locale, 'admin.phase.downloading')).first().waitFor()
    expect(installs).toEqual([{ force: false }])
    await ctx.close()
  })
})
