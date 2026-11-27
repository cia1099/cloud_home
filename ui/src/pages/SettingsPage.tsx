import { useState } from 'react'
import { Link } from 'react-router'
import { Icon } from '../components/Icon'
import { LanguageSwitch } from '../components/LanguageSwitch'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { APPEARANCES, type Appearance } from '../theme/theme'
import { useTheme } from '../theme/useTheme'

const APPEARANCE_LABELS: Record<Appearance, MessageKey> = {
  auto: 'settings.appearance.auto',
  light: 'settings.appearance.light',
  dark: 'settings.appearance.dark',
}

/** Foundation controls only: language, appearance, sign out. The full settings screen is a later task. */
export function SettingsPage() {
  const { t } = useI18n()
  const { session, signOut } = useSession()
  const { appearance, setAppearance } = useTheme()
  const [signingOut, setSigningOut] = useState(false)

  return (
    <div className="page">
      <h1 className="h1">{t('settings.title')}</h1>
      <section>
        <h2 className="group-label">{t('settings.account')}</h2>
        <div className="cell-list">
          <div className="cell">
            <span className="lbl">{session.phase === 'authed' ? session.user.username : ''}</span>
            <button
              type="button"
              className="btn plain"
              disabled={signingOut}
              onClick={() => {
                setSigningOut(true)
                void signOut()
              }}
            >
              {t('auth.sign_out')}
            </button>
          </div>
        </div>
      </section>
      <section>
        <h2 className="group-label">{t('settings.language')}</h2>
        <div className="cell-list">
          <div className="cell">
            <LanguageSwitch />
          </div>
        </div>
      </section>
      <section>
        <h2 className="group-label">{t('settings.appearance')}</h2>
        <div className="cell-list">
          <div className="cell">
            <div className="seg" role="group" aria-label={t('settings.appearance')}>
              {APPEARANCES.map((choice) => (
                <button key={choice} type="button" aria-pressed={choice === appearance} onClick={() => setAppearance(choice)}>
                  {t(APPEARANCE_LABELS[choice])}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>
      <section className="phone-only">
        <div className="cell-list">
          <Link className="cell" to="/storage">
            <span className="lbl">{t('nav.storage')}</span>
            <Icon name="chevR" size={16} />
          </Link>
        </div>
      </section>
      <p className="small num">
        {t('app.name')} {__APP_VERSION__}
      </p>
    </div>
  )
}
