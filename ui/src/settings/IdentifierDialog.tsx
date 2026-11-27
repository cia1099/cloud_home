import { useId, useState, type FormEvent } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import type { User } from '../api/auth'
import { removeIdentifier, sendChangeCode, setIdentifier } from '../api/profile'
import { PasswordField } from '../auth/PasswordField'
import { Dialog } from '../components/Dialog'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { byRegion, checkPhone, formatPhone, regionName, SUGGESTED, type Country } from './countries'
import { challengeTarget, identityOf, isCodeComplete, normalizeEmail, type Provider } from './model'
import { CodeStep, ErrorLine } from './Parts'
import { PhoneField } from './PhoneField'

export type IdentifierAction = 'add' | 'change' | 'remove'

interface Props {
  user: User
  provider: Provider
  action: IdentifierAction
  onDone: (user: User) => void
  onClose: () => void
}

type Failure = { field: 'value' | 'password' | 'code' | 'form'; message: MessageKey; params?: Record<string, string> }

/**
 * Add, change or remove an email or phone number: form (new value + current password), then the
 * one-time code the server sends to the existing identifier. Both factors are required by the API.
 */
export function IdentifierDialog({ user, provider, action, onDone, onClose }: Props) {
  const { t, locale } = useI18n()
  const id = useId()
  const [phase, setPhase] = useState<'form' | 'code'>('form')
  const [email, setEmail] = useState('')
  const [country, setCountry] = useState<Country>(() => byRegion(SUGGESTED[locale][0]))
  const [national, setNational] = useState('')
  const [password, setPassword] = useState('')
  const [code, setCode] = useState('')
  const [sentAt, setSentAt] = useState(0)
  const [busy, setBusy] = useState(false)
  const [failure, setFailure] = useState<Failure | null>(null)
  const [touchedPhone, setTouchedPhone] = useState(false)

  const current = identityOf(user, provider)?.identifier
  const target = challengeTarget(user, provider)
  const shown = (value: string) => (value.startsWith('+') ? formatPhone(value) : value)

  /** The normalized new value, or the reason it is not valid. */
  const newValue = (): { ok: true; value: string } | { ok: false; failure: Failure } => {
    if (provider === 'email') {
      const value = normalizeEmail(email)
      return value ? { ok: true, value } : { ok: false, failure: { field: 'value', message: 'account.id.email_invalid' } }
    }
    const check = checkPhone(country, national)
    return check.ok
      ? { ok: true, value: check.e164 }
      : { ok: false, failure: { field: 'value', message: 'account.id.phone_bad', params: { country: regionName(locale, country.region) } } }
  }

  const validatePhoneOnBlur = () => {
    setTouchedPhone(true)
    if (national.trim() === '') return
    const result = newValue()
    setFailure(result.ok ? null : result.failure)
  }

  const send = async () => {
    setBusy(true)
    try {
      await sendChangeCode(provider)
      setSentAt(Date.now())
      setPhase('code')
      setFailure(null)
    } catch (cause) {
      // The server uses one code for "too many codes" and for bad input; the form validated the input already.
      setFailure({ field: 'form', message: cause instanceof ApiError && cause.code === 'BAD_REQUEST' ? 'account.code.limit' : errorKey(cause) })
    } finally {
      setBusy(false)
    }
  }

  const submitForm = (event: FormEvent) => {
    event.preventDefault()
    if (action !== 'remove') {
      const result = newValue()
      if (!result.ok) {
        setFailure(result.failure)
        return
      }
    }
    void send()
  }

  const submitCode = async (event: FormEvent) => {
    event.preventDefault()
    if (!isCodeComplete(code)) {
      setFailure({ field: 'code', message: 'account.code.wrong' })
      return
    }
    setBusy(true)
    try {
      const result = newValue()
      const updated =
        action === 'remove'
          ? await removeIdentifier(provider, password, code)
          : await setIdentifier(provider, result.ok ? result.value : '', password, code)
      onDone(updated)
    } catch (cause) {
      setBusy(false)
      if (cause instanceof ApiError && cause.status === 401) {
        // Wrong current password: back to the form, where the password is.
        setPhase('form')
        setCode('')
        setFailure({ field: 'password', message: 'account.password.wrong' })
      } else if (cause instanceof ApiError && cause.status === 409) {
        setPhase('form')
        setCode('')
        setFailure({ field: 'value', message: provider === 'email' ? 'account.id.email_exists' : 'account.id.phone_exists' })
      } else if (cause instanceof ApiError && cause.status === 400) {
        setFailure({ field: 'code', message: 'account.code.wrong' })
      } else {
        setFailure({ field: 'form', message: errorKey(cause) })
      }
    }
  }

  const failureText = (field: Failure['field']) => (failure?.field === field ? t(failure.message, failure.params) : null)
  const title = t(`account.id.title.${action}.${provider}` as MessageKey)
  const confirmLabel = action === 'remove' ? t('account.remove') : t('account.code.confirm')

  return (
    <Dialog title={title} onClose={onClose} alert={action === 'remove'}>
      {phase === 'form' ? (
        <form className="dialog-form" onSubmit={submitForm} noValidate>
          {action === 'remove' ? (
            <p>{t('account.id.remove_text', { value: shown(current ?? '') })}</p>
          ) : provider === 'email' ? (
            <div className="field">
              <label htmlFor={`${id}-email`}>{t('auth.email')}</label>
              <input
                id={`${id}-email`}
                className="input"
                type="email"
                inputMode="email"
                autoComplete="email"
                autoCapitalize="none"
                spellCheck={false}
                placeholder="name@example.com"
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value)
                  if (failure?.field === 'value') setFailure(null)
                }}
                aria-invalid={failure?.field === 'value' || undefined}
                aria-describedby={`${id}-value-error`}
              />
              <ErrorLine id={`${id}-value-error`} message={failureText('value')} />
            </div>
          ) : (
            <>
              <PhoneField
                country={country}
                onCountry={setCountry}
                value={national}
                onChange={(value) => {
                  setNational(value)
                  if (failure?.field === 'value') setFailure(null)
                }}
                onBlur={validatePhoneOnBlur}
                invalid={touchedPhone && failure?.field === 'value'}
                describedBy={`${id}-value-error`}
              />
              <ErrorLine id={`${id}-value-error`} message={failureText('value')} />
            </>
          )}
          <PasswordField
            label="account.password.current"
            autoComplete="current-password"
            value={password}
            onChange={(value) => {
              setPassword(value)
              if (failure?.field === 'password') setFailure(null)
            }}
            invalid={failure?.field === 'password'}
            hint={t('account.id.why')}
          />
          <ErrorLine message={failureText('password') ?? failureText('form')} />
          <div className="row-end">
            <button type="button" className="btn plain" onClick={onClose}>
              {t('files.cancel')}
            </button>
            <button type="submit" className="btn primary" disabled={busy || password === ''}>
              {t('account.code.send')}
            </button>
          </div>
        </form>
      ) : (
        <form className="dialog-form" onSubmit={(event) => void submitCode(event)} noValidate>
          <CodeStep
            to={shown(target ?? '')}
            code={code}
            onCode={(value) => {
              setCode(value)
              if (failure?.field === 'code') setFailure(null)
            }}
            sentAt={sentAt}
            onResend={() => void send()}
            busy={busy}
            invalid={failure?.field === 'code'}
            errorId={`${id}-code-error`}
          />
          <ErrorLine id={`${id}-code-error`} message={failureText('code') ?? failureText('form')} />
          <div className="row-end">
            <button type="button" className="btn plain" onClick={onClose}>
              {t('files.cancel')}
            </button>
            <button type="submit" className={`btn ${action === 'remove' ? 'danger' : 'primary'}`} disabled={busy || !isCodeComplete(code)}>
              {confirmLabel}
            </button>
          </div>
        </form>
      )}
    </Dialog>
  )
}
