import { useId, useState, type FormEvent } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import type { User } from '../api/auth'
import { sendVerifyCode, verifyIdentifier } from '../api/profile'
import { Dialog } from '../components/Dialog'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { formatPhone } from './countries'
import { isCodeComplete, type Provider } from './model'
import { CodeStep, ErrorLine } from './Parts'

interface Props {
  provider: Provider
  /** The identifier the code was just sent to. */
  identifier: string
  sentAt: number
  onDone: (user: User) => void
  onClose: () => void
}

/** Confirms an identifier the account already has with a code sent to it (`POST /me/identifier/verify`). */
export function VerifyDialog({ provider, identifier, sentAt: firstSentAt, onDone, onClose }: Props) {
  const { t } = useI18n()
  const id = useId()
  const [code, setCode] = useState('')
  const [sentAt, setSentAt] = useState(firstSentAt)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<MessageKey | null>(null)

  const resend = async () => {
    setBusy(true)
    try {
      await sendVerifyCode(provider)
      setSentAt(Date.now())
      setError(null)
    } catch (cause) {
      setError(cause instanceof ApiError && cause.code === 'BAD_REQUEST' ? 'account.code.limit' : errorKey(cause))
    } finally {
      setBusy(false)
    }
  }

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setBusy(true)
    try {
      onDone(await verifyIdentifier(provider, code))
    } catch (cause) {
      setBusy(false)
      setError(cause instanceof ApiError && cause.status === 400 ? 'account.code.wrong' : errorKey(cause))
    }
  }

  return (
    <Dialog title={t(`account.verify.title.${provider}` as MessageKey)} onClose={onClose}>
      <form className="dialog-form" onSubmit={(event) => void submit(event)} noValidate>
        <CodeStep
          to={identifier.startsWith('+') ? formatPhone(identifier) : identifier}
          code={code}
          onCode={(value) => {
            setCode(value)
            setError(null)
          }}
          sentAt={sentAt}
          onResend={() => void resend()}
          busy={busy}
          invalid={error === 'account.code.wrong'}
          errorId={`${id}-error`}
        />
        <ErrorLine id={`${id}-error`} message={error ? t(error) : null} />
        <div className="row-end">
          <button type="button" className="btn plain" onClick={onClose}>
            {t('files.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={busy || !isCodeComplete(code)}>
            {t('account.verify')}
          </button>
        </div>
      </form>
    </Dialog>
  )
}
