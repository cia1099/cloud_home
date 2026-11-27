import { en, type MessageKey, type Messages } from './messages/en'
import { zhHans } from './messages/zh-Hans'
import { zhHant } from './messages/zh-Hant'

export const LOCALES = ['en', 'zh-Hans', 'zh-Hant'] as const
export type Locale = (typeof LOCALES)[number]

export const catalogs: Record<Locale, Messages> = { en, 'zh-Hans': zhHans, 'zh-Hant': zhHant }

/** Each language is always shown by its own name (design/app-spec.md section 2). */
export const LOCALE_NAMES: Record<Locale, string> = { en: 'English', 'zh-Hans': '简体中文', 'zh-Hant': '繁體中文' }

/** BCP 47 tags for `Intl`; the script subtag selects the right CJK conventions. */
export const INTL_TAGS: Record<Locale, string> = { en: 'en', 'zh-Hans': 'zh-Hans-CN', 'zh-Hant': 'zh-Hant-TW' }

const STORAGE_KEY = 'lang'

export function isLocale(value: unknown): value is Locale {
  return LOCALES.includes(value as Locale)
}

/** R-005: zh-TW/HK/MO/Hant -> Traditional; other zh (CN, SG, Hans, bare) -> Simplified; otherwise English. */
export function detectLocale(languages: readonly string[]): Locale {
  for (const tag of languages) {
    const parts = tag.toLowerCase().split('-')
    if (parts[0] === 'zh') {
      return parts.includes('hant') || parts.some((p) => p === 'tw' || p === 'hk' || p === 'mo') ? 'zh-Hant' : 'zh-Hans'
    }
    if (parts[0] === 'en') return 'en'
  }
  return 'en'
}

export function initialLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY)
  return isLocale(stored) ? stored : detectLocale(navigator.languages)
}

export function rememberLocale(locale: Locale): void {
  localStorage.setItem(STORAGE_KEY, locale)
}

export function translate(locale: Locale, key: MessageKey, params?: Record<string, string | number>): string {
  const text = catalogs[locale][key]
  if (!params) return text
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in params ? String(params[name]) : whole))
}

const UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const

/** Decimal units (kB/MB/GB) like the design mockup, localized through Intl. */
export function formatBytes(locale: Locale, bytes: number): string {
  const exp = bytes > 0 ? Math.min(Math.floor(Math.log10(bytes) / 3), UNITS.length - 1) : 0
  const value = bytes / 1000 ** exp
  return new Intl.NumberFormat(INTL_TAGS[locale], {
    style: 'unit',
    unit: UNITS[exp],
    unitDisplay: 'short',
    maximumFractionDigits: exp === 0 || value >= 100 ? 0 : 1,
  }).format(value)
}
