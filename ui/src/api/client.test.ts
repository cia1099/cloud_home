import { afterEach, describe, expect, it, vi } from 'vitest'
import { api, ApiError, setUnauthorizedHandler } from './client'
import { errorKey } from './errors'

function respond(status: number, body: unknown): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(body === undefined ? null : JSON.stringify(body), { status })),
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
  setUnauthorizedHandler(undefined)
})

describe('api', () => {
  it('requests a relative /api/v1 URL with same-origin credentials and unwraps data', async () => {
    respond(200, { data: { ok: 1 } })
    await expect(api('/auth/me')).resolves.toEqual({ ok: 1 })
    const [url, init] = vi.mocked(fetch).mock.calls[0]
    expect(url).toBe('/api/v1/auth/me')
    expect(init?.credentials).toBe('same-origin')
  })

  it('sends JSON bodies', async () => {
    respond(200, { data: null })
    await api('/auth/login', { method: 'POST', body: { a: 1 } })
    const init = vi.mocked(fetch).mock.calls[0][1]
    expect(init?.method).toBe('POST')
    expect(init?.body).toBe('{"a":1}')
    expect(init?.headers).toEqual({ 'Content-Type': 'application/json' })
  })

  it('turns the error envelope into an ApiError', async () => {
    respond(403, { error: { code: 'ACCOUNT_PENDING', message: 'pending' } })
    await expect(api('/auth/login')).rejects.toMatchObject({ status: 403, code: 'ACCOUNT_PENDING' })
  })

  it('uses UNKNOWN when the error body is not an envelope', async () => {
    respond(502, undefined)
    await expect(api('/x')).rejects.toMatchObject({ status: 502, code: 'UNKNOWN' })
  })

  it('reports an unreachable server as NETWORK', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')))
    await expect(api('/x')).rejects.toMatchObject({ status: 0, code: 'NETWORK' })
  })

  it('calls the unauthorized handler on 401', async () => {
    const handler = vi.fn()
    setUnauthorizedHandler(handler)
    respond(401, { error: { code: 'UNAUTHORIZED', message: 'no' } })
    await expect(api('/files')).rejects.toBeInstanceOf(ApiError)
    expect(handler).toHaveBeenCalledOnce()
  })
})

describe('errorKey', () => {
  it('maps known codes and falls back for unknown ones', () => {
    expect(errorKey(new ApiError(429, 'REGISTRATION_BUSY', ''))).toBe('error.REGISTRATION_BUSY')
    expect(errorKey(new ApiError(0, 'NETWORK', ''))).toBe('error.NETWORK')
    expect(errorKey(new ApiError(409, 'SOMETHING_NEW', ''))).toBe('error.UNKNOWN')
    expect(errorKey(new Error('boom'))).toBe('error.UNKNOWN')
  })
})
