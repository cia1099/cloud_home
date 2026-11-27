import { Navigate, Outlet, useLocation } from 'react-router'
import { errorKey } from '../api/errors'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { useSession } from '../session/useSession'
import { EventStreamProvider } from '../sse/EventStreamProvider'

/** Guards the signed-in app: loading, failed start, redirect to sign-in (remembering the route), or the app. */
export function RequireAuth() {
  const { t } = useI18n()
  const { session, retry } = useSession()
  const location = useLocation()

  if (session.phase === 'loading') {
    return (
      <div className="centered" role="status" aria-label={t('loading')}>
        <Icon name="spinner" size={32} className="spin" />
      </div>
    )
  }
  if (session.phase === 'error') {
    return (
      <div className="centered">
        <div className="state" role="alert">
          <span className="big">
            <Icon name="wifioff" size={44} />
          </span>
          <p>{t(errorKey(session.error))}</p>
          <button type="button" className="btn primary" onClick={retry}>
            {t('retry')}
          </button>
        </div>
      </div>
    )
  }
  if (session.phase === 'anon') {
    return <Navigate to="/signin" replace state={{ from: location.pathname + location.search }} />
  }
  return (
    <EventStreamProvider>
      <Outlet />
    </EventStreamProvider>
  )
}
