import { describe, expect, it } from 'vitest'
import { catalogs, detectLocale, formatBytes, formatDate, LOCALES, pluralKey, translate } from './locale'
import { en } from './messages/en'

describe('catalogs', () => {
  const keys = Object.keys(en).sort()
  it.each(LOCALES)('%s has exactly the English keys', (locale) => {
    expect(Object.keys(catalogs[locale]).sort()).toEqual(keys)
  })
  it.each(LOCALES)('%s has no empty text and keeps the placeholders of English', (locale) => {
    for (const key of keys as (keyof typeof en)[]) {
      const text = catalogs[locale][key]
      expect(text.trim(), `${locale} ${key}`).not.toBe('')
      const placeholders = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort()
      expect(placeholders(text), `${locale} ${key}`).toEqual(placeholders(en[key]))
    }
  })
})

describe('detectLocale', () => {
  it.each([
    [['zh-TW'], 'zh-Hant'],
    [['zh-HK'], 'zh-Hant'],
    [['zh-MO'], 'zh-Hant'],
    [['zh-Hant'], 'zh-Hant'],
    [['zh-Hant-CN'], 'zh-Hant'],
    [['zh-CN'], 'zh-Hans'],
    [['zh-SG'], 'zh-Hans'],
    [['zh-Hans'], 'zh-Hans'],
    [['zh'], 'zh-Hans'],
    [['en-US'], 'en'],
    [['fr-FR'], 'en'],
    [[], 'en'],
    [['fr-FR', 'zh-TW'], 'zh-Hant'],
  ] as const)('%j -> %s', (languages, expected) => {
    expect(detectLocale(languages)).toBe(expected)
  })
})

describe('translate', () => {
  it('fills placeholders and leaves unknown ones visible', () => {
    expect(translate('en', 'files.search.location', { path: 'Photos' })).toBe('In Photos')
    expect(translate('zh-Hant', 'files.search.results_for', {})).toBe('「{q}」的搜尋結果')
  })
})

describe('pluralKey', () => {
  it('uses the singular form in English and one form in Chinese', () => {
    expect(pluralKey('en', 'files.selected', 1)).toBe('files.selected.one')
    expect(pluralKey('en', 'files.selected', 3)).toBe('files.selected.other')
    expect(pluralKey('zh-Hans', 'files.selected', 1)).toBe('files.selected.other')
  })
})

describe('formatDate', () => {
  it('formats in the locale and tolerates bad input', () => {
    expect(formatDate('en', '2026-11-27T15:04:00Z')).toMatch(/2026/)
    expect(formatDate('zh-Hant', '2026-11-27T15:04:00Z')).toMatch(/2026/)
    expect(formatDate('en', 'nope')).toBe('')
  })
})

describe('formatBytes', () => {
  it('uses decimal units and the locale number format', () => {
    expect(formatBytes('en', 0)).toBe('0 byte')
    expect(formatBytes('en', 1_500_000)).toBe('1.5 MB')
    expect(formatBytes('en', 3_600_000_000)).toBe('3.6 GB')
    expect(formatBytes('zh-Hans', 2_000_000_000)).toMatch(/^2\s?GB$/)
  })
})
