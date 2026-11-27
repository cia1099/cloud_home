import { useId, useState, type FormEvent } from 'react'
import { Link, Navigate, useNavigate } from 'react-router'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { AuthLayout } from './AuthLayout'
import { OAuthButtons } from './OAuthButtons'
import { PasswordField } from './PasswordField'
import { StatusScreen, type StatusKind } from './StatusScreen'

type Method = 'email' | 'phone'

export function RegisterPage() {
  const { t } = useI18n()
  const { session, signUp } = useSession()
  const navigate = useNavigate()
  const id = useId()
  const [username, setUsername] = useState('')
  const [method, setMethod] = useState<Method>('email')
  const [identifier, setIdentifier] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [failure, setFailure] = useState<MessageKey | null>(null)
  const [status, setStatus] = useState<StatusKind | null>(null)

  if (session.phase === 'authed') return <Navigate to="/files" replace />

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setFailure(null)
    const value = identifier.trim()
    try {
      const outcome = await signUp({
        username: username.trim(),
        password,
        ...(method === 'email' ? { email: value } : { phone: value }),
      })
      // 201: the session is already set and the route guard moves on. 202: wait for the administrator.
      if (outcome === 'pending_approval') navigate('/pending', { replace: true })
    } catch (cause) {
      if (cause instanceof ApiError && cause.code === 'REGISTRATION_CLOSED') setStatus('closed')
      else if (cause instanceof ApiError && cause.code === 'REGISTRATION_BUSY') setStatus('busy')
      else setFailure(errorKey(cause))
    } finally {
      setBusy(false)
    }
  }

  if (status) {
    return (
      <AuthLayout>
        <StatusScreen kind={status} onBack={() => navigate('/signin')} />
      </AuthLayout>
    )
  }

  return (
    <AuthLayout>
      <h1>{t('auth.register.title')}</h1>
      <p className="sub">{t('auth.register.sub')}</p>
      <form onSubmit={submit}>
        <div className="field">
          <label htmlFor={`${id}-name`}>{t('auth.display_name')}</label>
          <input
            id={`${id}-name`}
            className="input"
            type="text"
            autoComplete="name"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
          />
        </div>
        <div className="seg" role="group" aria-label={t('auth.method')}>
          {(['email', 'phone'] as const).map((m) => (
            <button key={m} type="button" aria-pressed={m === method} onClick={() => setMethod(m)}>
              {t(m === 'email' ? 'auth.email' : 'auth.phone')}
            </button>
          ))}
        </div>
        <div className="field">
          <label htmlFor={`${id}-id`}>{t(method === 'email' ? 'auth.email' : 'auth.phone')}</label>
          <input
            id={`${id}-id`}
            className="input"
            type={method === 'email' ? 'email' : 'tel'}
            inputMode={method === 'email' ? 'email' : 'tel'}
            autoComplete={method === 'email' ? 'email' : 'tel'}
            autoCapitalize="none"
            spellCheck={false}
            placeholder={method === 'email' ? 'name@example.com' : '+1 555 0134'}
            value={identifier}
            onChange={(e) => setIdentifier(e.target.value)}
            required
          />
        </div>
        <PasswordField value={password} onChange={setPassword} autoComplete="new-password" hint={t('auth.password.hint')} />
        <div role="alert">
          {failure && (
            <p className="hint err">
              <Icon name="alert" size={16} />
              {t(failure)}
            </p>
          )}
        </div>
        <button type="submit" className="btn primary lg block" disabled={busy}>
          {t('auth.register')}
        </button>
      </form>
      <OAuthButtons />
      <p className="auth-foot">
        {t('auth.have_account')} <Link to="/signin">{t('auth.sign_in')}</Link>
      </p>
    </AuthLayout>
  )
}
