import { useId, useState, type FormEvent } from 'react'
import { Navigate } from 'react-router'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { UpdatePanel } from '../update/UpdatePanel'
import { useAdmin } from '../update/useAdmin'

/** The shared administrator password is asked for here; it is kept in memory for this tab only. */
function PasswordGate() {
  const { t } = useI18n()
  const { unlock } = useAdmin()
  const id = useId()
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const submit = (event: FormEvent) => {
    event.preventDefault()
    setBusy(true)
    setError(null)
    unlock(password).catch((cause: unknown) => {
      setBusy(false)
      setError(cause instanceof ApiError && cause.status === 401 ? t('admin.password_wrong') : t(errorKey(cause)))
    })
  }

  return (
    <form className="upd-gate" onSubmit={submit}>
      <span className="upd-logo" aria-hidden="true">
        <Icon name="shield" size={34} />
      </span>
      <h2>{t('admin.gate.title')}</h2>
      <p className="hint">{t('admin.gate.text')}</p>
      <div className="field">
        <label htmlFor={id}>{t('admin.password')}</label>
        <input
          id={id}
          className="input"
          type="password"
          autoComplete="off"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-err` : undefined}
          required
          autoFocus
        />
        {error && (
          <p id={`${id}-err`} className="hint err" role="alert">
            <Icon name="alert" size={14} />
            {error}
          </p>
        )}
      </div>
      <button type="submit" className="btn primary lg block" disabled={busy || password === ''}>
        {t('admin.continue')}
      </button>
    </form>
  )
}

/** Administration (design/app-spec.md section 9): one product version, install, mode, window, channel. */
export function AdminPage() {
  const { t } = useI18n()
  const { access } = useAdmin()

  if (access.state === 'unavailable') return <Navigate to="/settings" replace />
  return (
    <div className="page">
      <h1 className="h1">{t('admin.title')}</h1>
      {access.state === 'checking' && (
        <div className="centered" role="status" aria-label={t('loading')}>
          <Icon name="spinner" size={32} className="spin" />
        </div>
      )}
      {access.state === 'locked' && <PasswordGate />}
      {access.state === 'unlocked' && <UpdatePanel password={access.password} first={access.first} />}
    </div>
  )
}
