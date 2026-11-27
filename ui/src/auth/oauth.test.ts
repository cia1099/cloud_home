import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { authorizeUrl, fetchProviders, oauthLogin } from '../api/oauth'
import { beginState, checkCallback, redirectUriFor } from './oauthFlow'

function respond(status: number, body: unknown): void {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(body), { status })))
}

afterEach(() => vi.unstubAllGlobals())

function memoryStorage() {
  const map = new Map<string, string>()
  return {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
    removeItem: (k: string) => void map.delete(k),
    size: () => map.size,
  }
}

describe('fetchProviders', () => {
  it('is empty when OAuth is off or no provider is configured, so no button is drawn', async () => {
    respond(200, { data: { enabled: false, providers: ['google'] } })
    await expect(fetchProviders()).resolves.toEqual([])
    respond(200, { data: { enabled: true, providers: [] } })
    await expect(fetchProviders()).resolves.toEqual([])
  })

  it('lists only the configured known providers', async () => {
    respond(200, { data: { enabled: true, providers: ['apple', 'github'] } })
    await expect(fetchProviders()).resolves.toEqual(['apple'])
  })
})

describe('authorizeUrl and oauthLogin', () => {
  it('sends the redirect uri and state, and unwraps the url', async () => {
    respond(200, { data: { url: 'https://idp.example/auth' } })
    await expect(authorizeUrl('google', 'https://h/auth/callback/google', 'abc')).resolves.toBe('https://idp.example/auth')
    const url = String(vi.mocked(fetch).mock.calls[0][0])
    expect(url).toContain('/api/v1/auth/oauth/google/authorize?')
    expect(new URLSearchParams(url.split('?')[1]).get('state')).toBe('abc')
  })

  it('treats 202 as pending approval and 200 as signed in', async () => {
    respond(202, { data: { status: 'pending_approval' } })
    await expect(oauthLogin('google', 'c', 'r')).resolves.toEqual({ status: 'pending_approval' })
    respond(200, { data: { user: { id: 'u' } } })
    await expect(oauthLogin('google', 'c', 'r')).resolves.toMatchObject({ status: 'signed_in' })
    expect(JSON.parse(String(vi.mocked(fetch).mock.calls[0][1]?.body))).toEqual({ code: 'c', redirect_uri: 'r' })
  })

  it('surfaces the API error code with a localized message', async () => {
    respond(409, { error: { code: 'OAUTH_EMAIL_UNVERIFIED_LINK', message: 'x' } })
    const error = await oauthLogin('apple', 'c', 'r').catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect(errorKey(error)).toBe('error.OAUTH_EMAIL_UNVERIFIED_LINK')
  })
})

describe('state', () => {
  it('accepts a matching state once and then forgets it', () => {
    const storage = memoryStorage()
    const state = beginState(storage, 'google')
    expect(state).toMatch(/^[0-9a-f]{32}$/)
    expect(checkCallback(storage, 'google', `?code=C&state=${state}`)).toEqual({ ok: true, code: 'C' })
    expect(storage.size()).toBe(0)
    expect(checkCallback(storage, 'google', `?code=C&state=${state}`)).toEqual({ ok: false, reason: 'state' })
  })

  it('rejects a mismatching, missing or foreign-provider state', () => {
    for (const [provider, search] of [
      ['google', '?code=C&state=other'],
      ['google', '?code=C'],
      ['apple', 'PLACEHOLDER'],
    ] as const) {
      const storage = memoryStorage()
      const state = beginState(storage, 'google')
      const query = search === 'PLACEHOLDER' ? `?code=C&state=${state}` : search
      expect(checkCallback(storage, provider, query)).toEqual({ ok: false, reason: 'state' })
    }
  })

  it('reports a cancelled sign-in when the state is right but there is no code', () => {
    const storage = memoryStorage()
    const state = beginState(storage, 'apple')
    expect(checkCallback(storage, 'apple', `?error=access_denied&state=${state}`)).toEqual({ ok: false, reason: 'denied' })
  })

  it('builds the callback uri from the origin', () => {
    expect(redirectUriFor('https://box.local', 'apple')).toBe('https://box.local/auth/callback/apple')
  })
})
