import { useEffect, useRef, useState } from 'react'
import { Navigate, useLocation, useNavigate, useParams } from 'react-router'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { isOAuthProvider, type OAuthProvider } from '../api/oauth'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { AuthLayout } from './AuthLayout'
import { checkCallback, redirectUriFor } from './oauthFlow'
import { StatusScreen, type StatusKind } from './StatusScreen'

const STATUS_BY_CODE: Record<string, StatusKind> = {
  ACCOUNT_PENDING: 'account-pending',
  ACCOUNT_REJECTED: 'rejected',
  REGISTRATION_CLOSED: 'closed',
  REGISTRATION_BUSY: 'busy',
}

type Outcome = { kind: 'status'; status: StatusKind } | { kind: 'error'; message: MessageKey }

/** The provider sends the browser here with `code` and `state`; we verify, exchange, then enter the app. */
export function OAuthCallbackPage() {
  const { t } = useI18n()
  const { provider } = useParams()
  const { search } = useLocation()
  const navigate = useNavigate()
  const { session, signInWithOAuth } = useSession()
  const [outcome, setOutcome] = useState<Outcome | null>(null)
  // The code is single use and effects run twice in development: exchange exactly once per mount.
  const started = useRef(false)

  useEffect(() => {
    if (started.current || !isOAuthProvider(provider)) return
    started.current = true
    async function exchange(oauthProvider: OAuthProvider): Promise<Outcome | null> {
      const checked = checkCallback(sessionStorage, oauthProvider, search)
      if (!checked.ok) return { kind: 'error', message: checked.reason === 'state' ? 'oauth.error.state' : 'oauth.error.denied' }
      try {
        const result = await signInWithOAuth(oauthProvider, checked.code, redirectUriFor(window.location.origin, oauthProvider))
        if (result === 'pending_approval') navigate('/pending', { replace: true })
        return null
      } catch (cause) {
        const status = cause instanceof ApiError ? STATUS_BY_CODE[cause.code] : undefined
        return status ? { kind: 'status', status } : { kind: 'error', message: errorKey(cause) }
      }
    }
    void exchange(provider).then((result) => result && setOutcome(result))
  }, [provider, search, signInWithOAuth, navigate])

  if (!isOAuthProvider(provider)) return <Navigate to="/signin" replace />
  if (session.phase === 'authed') return <Navigate to="/files" replace />

  const back = () => navigate('/signin', { replace: true })
  return (
    <AuthLayout>
      {outcome?.kind === 'status' ? (
        <StatusScreen kind={outcome.status} onBack={back} />
      ) : outcome ? (
        <>
          <p className="hint err" role="alert">
            <Icon name="alert" size={16} />
            {t(outcome.message)}
          </p>
          <button type="button" className="btn plain block lg" onClick={back}>
            {t('approval.back_to_sign_in')}
          </button>
        </>
      ) : (
        <p className="sub" role="status">
          {t('oauth.signing_in')}
        </p>
      )}
    </AuthLayout>
  )
}
