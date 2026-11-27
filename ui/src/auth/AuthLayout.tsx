import type { ReactNode } from 'react'
import { Icon } from '../components/Icon'
import { LanguageSwitch } from '../components/LanguageSwitch'
import { useI18n } from '../i18n/useI18n'

/** Centered card with the language switch top-end, shared by every auth and status screen. */
export function AuthLayout({ children }: { children: ReactNode }) {
  const { t } = useI18n()
  return (
    <div className="auth">
      <LanguageSwitch className="langseg" />
      <main className="auth-card" aria-label={t('app.name')}>
        <span className="logo" aria-hidden="true">
          <Icon name="cloud" size={32} />
        </span>
        {children}
      </main>
    </div>
  )
}
