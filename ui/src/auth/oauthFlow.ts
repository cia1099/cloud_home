import type { OAuthProvider } from '../api/oauth'

const KEY = 'oauth_state'

/** Where the provider sends the browser back to; must match what the provider console lists. */
export const redirectUriFor = (origin: string, provider: OAuthProvider) => `${origin}/auth/callback/${provider}`

/** Creates a random `state`, remembers it for this tab only and returns it. */
export function beginState(storage: Pick<Storage, 'setItem'>, provider: OAuthProvider): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16))
  const state = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  storage.setItem(KEY, JSON.stringify({ state, provider }))
  return state
}

export type CallbackResult = { ok: true; code: string } | { ok: false; reason: 'denied' | 'state' }

/** Checks the provider's redirect query against the stored `state`. The stored value is single use. */
export function checkCallback(storage: Pick<Storage, 'getItem' | 'removeItem'>, provider: OAuthProvider, search: string): CallbackResult {
  const raw = storage.getItem(KEY)
  storage.removeItem(KEY)
  let stored: { state?: unknown; provider?: unknown } | null = null
  try {
    stored = raw === null ? null : (JSON.parse(raw) as { state?: unknown; provider?: unknown } | null)
  } catch {
    stored = null
  }
  const params = new URLSearchParams(search)
  if (!stored || typeof stored.state !== 'string' || stored.provider !== provider || params.get('state') !== stored.state) {
    return { ok: false, reason: 'state' }
  }
  const code = params.get('code')
  return code ? { ok: true, code } : { ok: false, reason: 'denied' }
}
