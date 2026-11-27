import { useState } from 'react'
import { authorizeUrl, fetchProviders, type OAuthProvider } from '../api/oauth'
import { errorKey } from '../api/errors'
import { Icon } from '../components/Icon'
import { useRemote } from '../data/useRemote'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { beginState, redirectUriFor } from './oauthFlow'

const LABEL: Record<OAuthProvider, MessageKey> = {
  google: 'oauth.continue_google',
  apple: 'oauth.continue_apple',
}

/** "Continue with …" buttons for the providers the server lists; renders nothing when there are none. */
export function OAuthButtons() {
  const { t } = useI18n()
  const { state } = useRemote(fetchProviders)
  const [busy, setBusy] = useState(false)
  const [failure, setFailure] = useState<MessageKey | null>(null)

  if (state.status !== 'ready' || state.data.length === 0) return null

  async function start(provider: OAuthProvider) {
    setBusy(true)
    setFailure(null)
    try {
      const stateValue = beginState(sessionStorage, provider)
      window.location.assign(await authorizeUrl(provider, redirectUriFor(window.location.origin, provider), stateValue))
    } catch (cause) {
      setFailure(errorKey(cause))
      setBusy(false)
    }
  }

  return (
    <div className="oauth">
      <p className="oauth-or" aria-hidden="true">
        {t('oauth.or')}
      </p>
      {state.data.map((provider) => (
        <button key={provider} type="button" className="btn plain lg block" disabled={busy} onClick={() => void start(provider)}>
          {t(LABEL[provider])}
        </button>
      ))}
      <div role="alert">
        {failure && (
          <p className="hint err">
            <Icon name="alert" size={16} />
            {t(failure)}
          </p>
        )}
      </div>
    </div>
  )
}
