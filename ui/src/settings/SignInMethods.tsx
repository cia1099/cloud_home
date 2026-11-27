import { useState } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { fetchMe, type User } from '../api/auth'
import { sendVerifyCode } from '../api/profile'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useSession } from '../session/useSession'
import { useToast } from '../toast/useToast'
import { formatPhone } from './countries'
import { IdentifierDialog, type IdentifierAction } from './IdentifierDialog'
import { canRemove, identityOf, type Provider } from './model'
import { VerifyDialog } from './VerifyDialog'

const PROVIDERS: readonly Provider[] = ['email', 'phone']

type Open =
  | { kind: 'identifier'; provider: Provider; action: IdentifierAction }
  | { kind: 'verify'; provider: Provider; identifier: string; sentAt: number }

const providerName = (provider: string) => provider.charAt(0).toUpperCase() + provider.slice(1)

/** Email and phone: verified state, and add / change / remove / verify through the one-time-code dialogs. */
export function SignInMethods({ user }: { user: User }) {
  const { t } = useI18n()
  const { setUser } = useSession()
  const toast = useToast()
  const [open, setOpen] = useState<Open | null>(null)
  const [sending, setSending] = useState<Provider | null>(null)

  const startVerify = async (provider: Provider, identifier: string) => {
    setSending(provider)
    try {
      await sendVerifyCode(provider)
      setOpen({ kind: 'verify', provider, identifier, sentAt: Date.now() })
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 409) {
        // Verified elsewhere in the meantime: just show the current state.
        await fetchMe().then(setUser, () => undefined)
      } else {
        toast({ text: t(cause instanceof ApiError && cause.code === 'BAD_REQUEST' ? 'account.code.limit' : errorKey(cause)) })
      }
    } finally {
      setSending(null)
    }
  }

  const done = (updated: User, text: MessageKey) => {
    setUser(updated)
    setOpen(null)
    toast({ text: t(text) })
  }

  const unverifiedAfterChange = (updated: User, provider: Provider) => identityOf(updated, provider)?.verified === false

  const lastOne = user.identities.filter((i) => i.provider === 'email' || i.provider === 'phone').length <= 1

  return (
    <>
      <div className="cell-list">
        {PROVIDERS.map((provider) => {
          const identity = identityOf(user, provider)
          const label = t(provider === 'email' ? 'auth.email' : 'auth.phone')
          const value = identity ? (provider === 'phone' ? formatPhone(identity.identifier) : identity.identifier) : null
          const managed = identity?.managed_by ?? null
          return (
            <div className="cell method" key={provider}>
              <Icon name={provider === 'email' ? 'mail' : 'phone'} size={20} />
              <div className="method-text">
                <span className="lbl">{label}</span>
                <span className="dim method-value" dir="auto">
                  {value ?? t('account.not_set')}
                </span>
                {managed && <span className="hint">{t('account.managed_by', { provider: providerName(managed) })}</span>}
              </div>
              {identity &&
                (identity.verified ? (
                  <span className="chip ok">
                    <Icon name="check" size={14} />
                    {t('account.verified')}
                  </span>
                ) : (
                  <span className="chip warn">
                    <Icon name="alert" size={14} />
                    {t('account.not_verified')}
                  </span>
                ))}
              {!managed && (
                <div className="method-actions">
                  {!identity && (
                    <button type="button" className="btn plain" onClick={() => setOpen({ kind: 'identifier', provider, action: 'add' })}>
                      {t('account.add')}
                    </button>
                  )}
                  {identity && !identity.verified && (
                    <button type="button" className="btn primary" disabled={sending === provider} onClick={() => void startVerify(provider, identity.identifier)}>
                      {t('account.verify')}
                    </button>
                  )}
                  {identity && (
                    <button type="button" className="btn plain" onClick={() => setOpen({ kind: 'identifier', provider, action: 'change' })}>
                      {t('account.change')}
                    </button>
                  )}
                  {identity && canRemove(user, provider) && (
                    <button type="button" className="btn plain danger-text" onClick={() => setOpen({ kind: 'identifier', provider, action: 'remove' })}>
                      {t('account.remove')}
                    </button>
                  )}
                </div>
              )}
            </div>
          )
        })}
      </div>
      {lastOne && <p className="hint pad-s">{t('account.keep_one')}</p>}

      {open?.kind === 'identifier' && (
        <IdentifierDialog
          user={user}
          provider={open.provider}
          action={open.action}
          onClose={() => setOpen(null)}
          onDone={(updated) => done(updated, open.action !== 'remove' && unverifiedAfterChange(updated, open.provider) ? 'account.saved_verify' : 'account.saved')}
        />
      )}
      {open?.kind === 'verify' && (
        <VerifyDialog
          provider={open.provider}
          identifier={open.identifier}
          sentAt={open.sentAt}
          onClose={() => setOpen(null)}
          onDone={(updated) => done(updated, 'account.verify.done')}
        />
      )}
    </>
  )
}
