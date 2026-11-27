import { api } from './client'
import type { RegisterResult, User } from './auth'

export const OAUTH_PROVIDERS = ['google', 'apple'] as const
export type OAuthProvider = (typeof OAUTH_PROVIDERS)[number]

export const isOAuthProvider = (value: string | undefined): value is OAuthProvider =>
  OAUTH_PROVIDERS.some((provider) => provider === value)

/** The providers the server has configured, in our display order. Empty when OAuth is switched off. */
export async function fetchProviders(signal?: AbortSignal): Promise<OAuthProvider[]> {
  const result = await api<{ enabled: boolean; providers: string[] }>('/auth/providers', { signal })
  return result.enabled ? OAUTH_PROVIDERS.filter((provider) => result.providers.includes(provider)) : []
}

export async function authorizeUrl(provider: OAuthProvider, redirectUri: string, state: string): Promise<string> {
  const query = new URLSearchParams({ redirect_uri: redirectUri, state })
  const result = await api<{ url: string }>(`/auth/oauth/${provider}/authorize?${query}`)
  return result.url
}

/** `200` signs the user in; `202` means the new account waits for the administrator. */
export async function oauthLogin(provider: OAuthProvider, code: string, redirectUri: string): Promise<RegisterResult> {
  const result = await api<{ status: 'pending_approval' } | { user: User }>(`/auth/oauth/${provider}/login`, {
    method: 'POST',
    body: { code, redirect_uri: redirectUri },
  })
  return 'user' in result ? { status: 'signed_in', user: result.user } : { status: 'pending_approval' }
}
