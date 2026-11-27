import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { formatBytes, initialLocale, rememberLocale, translate, type Locale } from './locale'
import { I18nContext, type I18n } from './useI18n'

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(initialLocale)

  // The html lang attribute selects the CJK glyph variant (R-005).
  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  const setLocale = useCallback((next: Locale) => {
    rememberLocale(next)
    setLocaleState(next)
  }, [])

  const value = useMemo<I18n>(
    () => ({
      locale,
      setLocale,
      t: (key, params) => translate(locale, key, params),
      bytes: (n) => formatBytes(locale, n),
    }),
    [locale, setLocale],
  )
  return <I18nContext value={value}>{children}</I18nContext>
}
