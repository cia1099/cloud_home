import { useId, useState, type FormEvent } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { changePassword } from '../api/profile'
import { PasswordField } from '../auth/PasswordField'
import { Dialog } from '../components/Dialog'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { isPasswordLongEnough } from './model'
import { ErrorLine } from './Parts'

/** Current password plus a new one (`POST /me/password`). Other sessions are not signed out by the API. */
export function PasswordDialog({ onDone, onClose }: { onDone: () => void; onClose: () => void }) {
  const { t } = useI18n()
  const id = useId()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [busy, setBusy] = useState(false)
  const [failure, setFailure] = useState<{ field: 'current' | 'next'; message: MessageKey } | null>(null)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (!isPasswordLongEnough(next)) {
      setFailure({ field: 'next', message: 'account.password.short' })
      return
    }
    setBusy(true)
    try {
      await changePassword(current, next)
      onDone()
    } catch (cause) {
      setBusy(false)
      setFailure(
        cause instanceof ApiError && cause.status === 401
          ? { field: 'current', message: 'account.password.wrong' }
          : { field: 'next', message: errorKey(cause) },
      )
    }
  }

  return (
    <Dialog title={t('account.password.title')} onClose={onClose}>
      <form className="dialog-form" onSubmit={(event) => void submit(event)} noValidate>
        <PasswordField
          label="account.password.current"
          autoComplete="current-password"
          value={current}
          onChange={(value) => {
            setCurrent(value)
            setFailure(null)
          }}
          invalid={failure?.field === 'current'}
        />
        <ErrorLine id={`${id}-current`} message={failure?.field === 'current' ? t(failure.message) : null} />
        <PasswordField
          label="account.password.new"
          autoComplete="new-password"
          value={next}
          onChange={(value) => {
            setNext(value)
            setFailure(null)
          }}
          invalid={failure?.field === 'next'}
          hint={t('auth.password.hint')}
        />
        <ErrorLine id={`${id}-next`} message={failure?.field === 'next' ? t(failure.message) : null} />
        <div className="row-end">
          <button type="button" className="btn plain" onClick={onClose}>
            {t('files.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={busy || current === '' || next === ''}>
            {t('account.name.save')}
          </button>
        </div>
      </form>
    </Dialog>
  )
}
