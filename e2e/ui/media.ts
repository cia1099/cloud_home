// Screenshots / recordings are only written when E2E_MEDIA_DIR is set (scripts/e2e_ui.sh --media),
// so a normal run leaves the tracked media files alone.
import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import type { Page } from 'playwright-core'

export const MEDIA_DIR = process.env.E2E_MEDIA_DIR ?? ''

export async function shot(page: Page, name: string): Promise<void> {
  if (!MEDIA_DIR) return
  mkdirSync(MEDIA_DIR, { recursive: true })
  await page.waitForTimeout(700) // let dialog / panel transitions and thumbnails finish
  await page.screenshot({ path: join(MEDIA_DIR, `${name}.png`) })
}
