import { useEffect, useId, useState } from 'react'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { cleanCode, secondsLeft } from './model'

/** Inline error: `role=alert` so it is announced when it appears. */
export function ErrorLine({ id, message }: { id?: string; message: string | null }) {
  return (
    <div id={id} role="alert">
      {message && (
        <p className="hint err">
          <Icon name="alert" size={14} />
          {message}
        </p>
      )}
    </div>
  )
}

/** Seconds until a new code may be sent, counted from `sentAt`. */
function useResendSeconds(sentAt: number): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(timer)
  }, [sentAt])
  // After a resend `now` can lag by up to a second; never count more than the full wait.
  return secondsLeft(sentAt, Math.max(now, sentAt))
}

interface CodeStepProps {
  to: string
  code: string
  onCode: (code: string) => void
  sentAt: number
  onResend: () => void
  busy: boolean
  invalid: boolean
  errorId: string
}

/** The six-digit field with the "sent to" line and the resend button with its countdown. */
export function CodeStep({ to, code, onCode, sentAt, onResend, busy, invalid, errorId }: CodeStepProps) {
  const { t } = useI18n()
  const id = useId()
  const left = useResendSeconds(sentAt)
  return (
    <>
      <p id={`${id}-sent`} className="hint" role="status">
        {t('account.code.sent_to', { to })}
      </p>
      <div className="field">
        <label htmlFor={`${id}-code`}>{t('account.code.label')}</label>
        <input
          id={`${id}-code`}
          className="input otp"
          // The user just chose to send a code: the field that takes it is the next thing they need.
          autoFocus
          value={code}
          onChange={(e) => onCode(cleanCode(e.target.value))}
          autoComplete="one-time-code"
          inputMode="numeric"
          pattern="[0-9]*"
          maxLength={7}
          spellCheck={false}
          aria-invalid={invalid || undefined}
          aria-describedby={`${id}-sent ${errorId}`}
        />
      </div>
      <button type="button" className="btn plain resend" disabled={busy || left > 0} onClick={onResend}>
        {left > 0 ? t('account.code.resend_in', { s: left }) : t('account.code.resend')}
      </button>
    </>
  )
}
