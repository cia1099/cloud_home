import { useId, useState, type FormEvent } from 'react'
import { errorKey } from '../api/errors'
import type { User } from '../api/auth'
import { updateName } from '../api/profile'
import { useI18n } from '../i18n/useI18n'
import { useSession } from '../session/useSession'
import { useToast } from '../toast/useToast'
import { checkName } from './model'
import { ErrorLine } from './Parts'

/** Display name: an inline field with Save (`PATCH /me`). */
export function ProfileSection({ user }: { user: User }) {
  const { t } = useI18n()
  const { setUser } = useSession()
  const toast = useToast()
  const id = useId()
  const [name, setName] = useState(user.username)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const changed = name.trim() !== user.username

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const check = checkName(name)
    if (check !== 'ok') {
      setError(t(check === 'short' ? 'account.name.short' : 'account.name.long'))
      return
    }
    setBusy(true)
    try {
      const updated = await updateName(name.trim())
      setUser(updated)
      setName(updated.username)
      toast({ text: t('account.saved') })
    } catch (cause) {
      setError(t(errorKey(cause)))
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="cell-list name-form" onSubmit={(event) => void submit(event)} noValidate>
      <div className="cell">
        <div className="field grow">
          <label htmlFor={`${id}-name`}>{t('auth.display_name')}</label>
          <input
            id={`${id}-name`}
            className="input"
            autoComplete="name"
            value={name}
            onChange={(e) => {
              setName(e.target.value)
              setError(null)
            }}
            aria-invalid={error !== null || undefined}
            aria-describedby={`${id}-error`}
          />
          <ErrorLine id={`${id}-error`} message={error} />
        </div>
        <button type="submit" className="btn primary" disabled={busy || !changed}>
          {t('account.name.save')}
        </button>
      </div>
    </form>
  )
}
