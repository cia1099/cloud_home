import { useI18n } from '../i18n/useI18n'
import { LOCALES, LOCALE_NAMES } from '../i18n/locale'

/** Three-way segmented control; each language is written in its own name and carries its own `lang`. */
export function LanguageSwitch({ className }: { className?: string }) {
  const { locale, setLocale, t } = useI18n()
  return (
    <div className={`seg ${className ?? ''}`} role="group" aria-label={t('settings.language')}>
      {LOCALES.map((code) => (
        <button key={code} type="button" lang={code} aria-pressed={code === locale} onClick={() => setLocale(code)}>
          {LOCALE_NAMES[code]}
        </button>
      ))}
    </div>
  )
}
