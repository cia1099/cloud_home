import { useRef, useState, type FormEvent } from 'react'
import { Link, Navigate, useLocation } from 'react-router'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { AuthLayout } from './AuthLayout'
import { PasswordField } from './PasswordField'
import { redirectTarget } from './redirect'
import { StatusScreen, type StatusKind } from './StatusScreen'

export function SignInPage() {
  const { t } = useI18n()
  const { session, signIn } = useSession()
  const location = useLocation()
  const [identifier, setIdentifier] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [failure, setFailure] = useState<'credentials' | MessageKey | null>(null)
  const [status, setStatus] = useState<StatusKind | null>(null)
  const identifierRef = useRef<HTMLInputElement>(null)

  if (session.phase === 'authed') return <Navigate to={redirectTarget(location.state)} replace />

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setFailure(null)
    try {
      await signIn({ identifier: identifier.trim(), password })
    } catch (cause) {
      // The same message on both fields for 401: never hint which one was wrong.
      if (cause instanceof ApiError && cause.code === 'ACCOUNT_PENDING') setStatus('account-pending')
      else if (cause instanceof ApiError && cause.code === 'ACCOUNT_REJECTED') setStatus('rejected')
      else if (cause instanceof ApiError && cause.status === 401) {
        setFailure('credentials')
        identifierRef.current?.focus()
      } else {
        setFailure(errorKey(cause))
      }
    } finally {
      setBusy(false)
    }
  }

  if (status) {
    return (
      <AuthLayout>
        <StatusScreen kind={status} onBack={() => setStatus(null)} />
      </AuthLayout>
    )
  }

  const invalid = failure === 'credentials'
  return (
    <AuthLayout>
      <h1>{t('auth.sign_in.title')}</h1>
      <p className="sub">{t('auth.sign_in.sub')}</p>
      {session.phase === 'anon' && session.expired && (
        <p className="notice" role="status">
          <Icon name="alert" size={18} />
          {t('session.expired')}
        </p>
      )}
      <form onSubmit={submit}>
        <div className="field">
          <label htmlFor="identifier">{t('auth.identifier')}</label>
          <input
            id="identifier"
            ref={identifierRef}
            className="input"
            type="text"
            inputMode="email"
            autoComplete="username"
            autoCapitalize="none"
            spellCheck={false}
            placeholder={t('auth.identifier.placeholder')}
            value={identifier}
            onChange={(e) => setIdentifier(e.target.value)}
            aria-invalid={invalid || undefined}
            required
          />
        </div>
        <PasswordField value={password} onChange={setPassword} invalid={invalid} autoComplete="current-password" />
        <div role="alert">
          {failure && (
            <p className="hint err">
              <Icon name="alert" size={16} />
              {t(invalid ? 'auth.wrong_credentials' : failure)}
            </p>
          )}
        </div>
        <button type="submit" className="btn primary lg block" disabled={busy}>
          {t('auth.sign_in')}
        </button>
      </form>
      <p className="auth-foot">
        {t('auth.no_account')} <Link to="/register">{t('auth.register')}</Link>
      </p>
    </AuthLayout>
  )
}
