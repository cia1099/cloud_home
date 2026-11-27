import { useState } from 'react'
import { Link } from 'react-router'
import { Icon } from '../components/Icon'
import { LanguageSwitch } from '../components/LanguageSwitch'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { APPEARANCES, type Appearance } from '../theme/theme'
import { useTheme } from '../theme/useTheme'
import { useToast } from '../toast/useToast'
import { PasswordDialog } from '../settings/PasswordDialog'
import { ProfileSection } from '../settings/ProfileSection'
import { SignInMethods } from '../settings/SignInMethods'

const APPEARANCE_LABELS: Record<Appearance, MessageKey> = {
  auto: 'settings.appearance.auto',
  light: 'settings.appearance.light',
  dark: 'settings.appearance.dark',
}

/** Profile, ways to sign in and password (R-012), then language, appearance and sign out. */
export function SettingsPage() {
  const { t } = useI18n()
  const { session, signOut } = useSession()
  const { appearance, setAppearance } = useTheme()
  const toast = useToast()
  const [signingOut, setSigningOut] = useState(false)
  const [changingPassword, setChangingPassword] = useState(false)
  const user = session.phase === 'authed' ? session.user : null

  return (
    <div className="page">
      <h1 className="h1">{t('settings.title')}</h1>
      {user && (
        <>
          <section>
            <h2 className="group-label">{t('settings.profile')}</h2>
            <ProfileSection key={user.id} user={user} />
          </section>
          <section>
            <h2 className="group-label">{t('account.methods')}</h2>
            <SignInMethods user={user} />
          </section>
          <section>
            <div className="cell-list">
              <div className="cell">
                <span className="lbl">{t(user.has_password ? 'account.password.title' : 'account.password.none')}</span>
                {user.has_password && (
                  <button type="button" className="btn plain" onClick={() => setChangingPassword(true)}>
                    {t('account.change')}
                  </button>
                )}
              </div>
            </div>
          </section>
          {changingPassword && (
            <PasswordDialog
              onClose={() => setChangingPassword(false)}
              onDone={() => {
                setChangingPassword(false)
                toast({ text: t('account.password.changed') })
              }}
            />
          )}
        </>
      )}
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
      <section>
        <h2 className="group-label">{t('settings.account')}</h2>
        <div className="cell-list">
          <div className="cell">
            <span className="lbl">{user?.username}</span>
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
      <p className="small num">
        {t('app.name')} {__APP_VERSION__}
      </p>
    </div>
  )
}
