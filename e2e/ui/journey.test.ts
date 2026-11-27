import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { mkdirSync, renameSync } from 'node:fs'
import { join } from 'node:path'
import type { Browser, BrowserContext, Page } from 'playwright-core'
import { LOCALES, translate, type Locale } from '../../ui/src/i18n/locale'
import type { MessageKey } from '../../ui/src/i18n/messages/en'
import { ADMIN_PASSWORD, launch, startServer, USER_PASSWORD, type Server } from './harness'
import { MEDIA_DIR, shot } from './media'
import { gradientPng } from './png'

const SCHEMES = ['light', 'dark'] as const
type Scheme = (typeof SCHEMES)[number]
/** Browser language that matches each UI language (not stored: the choice is made by detection). */
const BROWSER_LANG: Record<Locale, string> = { en: 'en-US', 'zh-Hans': 'zh-CN', 'zh-Hant': 'zh-TW' }
/** One recorded journey per language, alternating the colour scheme. */
const RECORDED: ReadonlySet<string> = new Set(['en/light', 'zh-Hans/dark', 'zh-Hant/light'])

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

const NOTE_TEXT = 'Hello from the e2e journey\n'

function tr(locale: Locale) {
  return (key: MessageKey, params?: Record<string, string | number>) => translate(locale, key, params)
}

async function newContext(locale: Locale, scheme: Scheme, record: string | null): Promise<BrowserContext> {
  return browser.newContext({
    locale: BROWSER_LANG[locale],
    colorScheme: scheme,
    viewport: { width: 1280, height: 800 },
    acceptDownloads: true,
    ...(record ? { recordVideo: { dir: join(server.dir, 'video-' + record), size: { width: 1280, height: 800 } } } : {}),
  })
}

describe.each(LOCALES)('journey in %s', (locale) => {
  describe.each(SCHEMES)('%s scheme', (scheme) => {
    test('register, upload, browse, preview, download, trash, restore, share, settings, admin', async () => {
      const t = tr(locale)
      const tag = `${locale}-${scheme}`
      const key = `${locale}/${scheme}`
      const record = RECORDED.has(key) && MEDIA_DIR ? tag : null
      const ctx = await newContext(locale, scheme, record)
      ctx.setDefaultTimeout(10_000)
      const page = await ctx.newPage()
      const errors: string[] = []
      page.on('pageerror', (e) => errors.push(e.message))
      // 401 is the expected answer of the signed-out session probe and of the admin probe
      page.on('console', (m) => m.type() === 'error' && !m.text().includes('status of 401') && errors.push(m.text()))

      const email = `e2e-${tag.toLowerCase()}@example.test`
      const photo = locale === 'en' ? 'holiday.png' : 'holiday-假期.png'
      const folder = locale === 'en' ? 'Trips' : '旅行'

      // --- register: the page is in the detected language and the scheme follows the system
      await page.goto(server.url + '/register')
      await page.getByRole('heading', { name: t('auth.register.title') }).waitFor()
      expect(await page.locator('html').getAttribute('lang')).toBe(locale)
      await page.getByLabel(t('auth.display_name')).fill(`Tester ${tag}`)
      await page.getByLabel(t('auth.email'), { exact: true }).fill(email)
      await page.getByLabel(t('auth.password'), { exact: true }).fill(USER_PASSWORD)
      await shot(page, `${tag}-01-register`)
      await page.getByRole('button', { name: t('auth.register'), exact: true }).click()

      // --- empty Files
      await page.getByText(t('files.empty.title')).waitFor()
      await shot(page, `${tag}-02-files-empty`)

      // --- sign out and sign in again with the same credentials
      await page.goto(server.url + '/settings')
      await page.getByRole('button', { name: t('auth.sign_out') }).click()
      await page.getByRole('heading', { name: t('auth.sign_in.title') }).waitFor()
      await shot(page, `${tag}-03-signin`)
      await page.getByLabel(t('auth.identifier')).fill(email)
      await page.getByLabel(t('auth.password'), { exact: true }).fill('wrong-password-1')
      await page.getByRole('button', { name: t('auth.sign_in'), exact: true }).click()
      await page.getByText(t('auth.wrong_credentials')).waitFor()
      await page.getByLabel(t('auth.password'), { exact: true }).fill(USER_PASSWORD)
      await page.getByRole('button', { name: t('auth.sign_in'), exact: true }).click()
      // signing in returns to the page the user was on (Settings)
      await page.getByRole('heading', { name: t('settings.title') }).waitFor()
      await page.getByRole('link', { name: t('nav.files') }).first().click()
      await page.getByText(t('files.empty.title')).waitFor()

      // --- new folder, open it
      await page.getByRole('button', { name: t('upload.menu') }).click()
      await page.getByRole('menuitem', { name: t('files.new_folder') }).click()
      await page.getByLabel(t('files.folder_name')).fill(folder)
      await page.getByRole('button', { name: t('files.create'), exact: true }).click()
      const folderRow = page.getByRole('option', { name: new RegExp(folder) })
      await folderRow.waitFor()
      await folderRow.dblclick()
      await page.waitForURL(/\/files\/.+/)

      // --- upload a text file and an image through the real file chooser
      await page.getByRole('button', { name: t('upload.menu') }).click()
      const chooser = page.waitForEvent('filechooser')
      await page.getByRole('menuitem', { name: t('upload.files'), exact: true }).click()
      await (await chooser).setFiles([
        { name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.from(NOTE_TEXT) },
        { name: photo, mimeType: 'image/png', buffer: gradientPng() },
      ])
      await page.getByText(t('upload.title.done')).waitFor()
      await shot(page, `${tag}-04-uploaded`)
      await page.getByRole('button', { name: t('upload.close') }).click()
      await page.getByRole('option', { name: /notes\.txt/ }).waitFor()
      await page.getByRole('option', { name: new RegExp(photo) }).waitFor()

      // --- preview the image
      await page.getByRole('option', { name: new RegExp(photo) }).dblclick()
      const preview = page.getByRole('dialog', { name: t('preview.label', { name: photo }) })
      await preview.waitFor()
      await preview.locator('img').first().waitFor()
      await shot(page, `${tag}-05-preview`)
      await page.keyboard.press('Escape')
      await preview.waitFor({ state: 'hidden' })

      // --- download the text file and compare its bytes
      const note = page.getByRole('option', { name: /notes\.txt/ })
      await note.click()
      const download = page.waitForEvent('download')
      await page.getByRole('button', { name: t('download.action'), exact: true }).click()
      const saved = await download
      expect(saved.suggestedFilename()).toBe('notes.txt')
      expect(await Bun.file((await saved.path()) as string).text()).toBe(NOTE_TEXT)

      // --- share the image with a link (downloads off), open it without a session
      await page.getByRole('option', { name: new RegExp(photo) }).click()
      await page.getByRole('button', { name: t('files.actions', { name: photo }) }).click()
      await page.getByRole('menuitem', { name: t('share.action') }).click()
      const dialog = page.getByRole('dialog')
      await dialog.waitFor()
      await dialog.getByRole('switch').uncheck({ force: true })
      await shot(page, `${tag}-06-share-dialog`)
      await dialog.getByRole('button', { name: t('share.create_link'), exact: true }).click()
      const link = await dialog.getByRole('textbox').inputValue()
      expect(link).toMatch(/\/s\/[\w-]+$/)
      await shot(page, `${tag}-07-share-link`)
      await dialog.getByRole('button', { name: t('share.done'), exact: true }).click()
      const guest = await newContext(locale, scheme, null)
      guest.setDefaultTimeout(10_000)
      const pub = await guest.newPage()
      await pub.goto(link)
      await pub.getByText(photo, { exact: false }).first().waitFor()
      await pub.getByText(t('public.note')).waitFor()
      await shot(pub, `${tag}-08-public-share`)
      await guest.close()

      // --- trash the text file, find it in Trash, restore it
      await page.getByRole('option', { name: /notes\.txt/ }).click()
      await page.getByRole('button', { name: t('files.actions', { name: 'notes.txt' }) }).click()
      await page.getByRole('menuitem', { name: t('files.delete') }).click()
      await page.getByRole('option', { name: /notes\.txt/ }).waitFor({ state: 'hidden' })
      await page.getByRole('link', { name: t('nav.trash') }).first().click()
      const trashed = page.getByText('notes.txt')
      await trashed.waitFor()
      await shot(page, `${tag}-09-trash`)
      await page.getByRole('button', { name: t('trash.restore') }).first().click()
      await trashed.waitFor({ state: 'hidden' })
      await page.getByText(t('trash.empty.title')).waitFor()
      await page.getByRole('link', { name: t('nav.files') }).first().click()
      await page.getByRole('option', { name: new RegExp(folder) }).dblclick()
      await page.getByRole('option', { name: /notes\.txt/ }).waitFor()

      // --- shared list shows the link; settings; admin update page
      await page.getByRole('link', { name: t('nav.shared') }).first().click()
      await page.getByText(photo).waitFor()
      await shot(page, `${tag}-10-shared`)
      await page.getByRole('link', { name: t('nav.settings') }).first().click()
      await page.getByRole('heading', { name: t('settings.title') }).waitFor()
      await shot(page, `${tag}-11-settings`)
      await page.getByRole('link', { name: t('nav.admin') }).click()
      await page.getByLabel(t('admin.password')).fill(ADMIN_PASSWORD)
      await page.getByRole('button', { name: t('admin.continue') }).click()
      await page.getByText(t('admin.software_update')).waitFor()
      // a dev run is not inside an install folder, so the page says updating is unavailable
      await page.getByText(t('admin.disabled.title')).waitFor()
      await page.waitForTimeout(1500)
      await shot(page, `${tag}-12-admin-update`)

      expect(errors).toEqual([])
      await ctx.close()
      if (record && MEDIA_DIR) {
        const video = page.video()
        mkdirSync(MEDIA_DIR, { recursive: true })
        if (video) renameSync(await video.path(), join(MEDIA_DIR, `journey-${tag}.webm`))
      }
    })
  })
})
