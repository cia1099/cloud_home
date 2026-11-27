/** All requests are relative to the page origin: a Vite proxy in development, the same binary in production. */
export const API_BASE = '/api/v1'

export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE'
  body?: unknown
  signal?: AbortSignal
  /** For requests that carry the current password: a 401 means "wrong password", not "session ended". */
  keepSession?: boolean
  /** Administration endpoints use the shared admin password, not the session: a 401 there must not sign the user out. */
  adminPassword?: string
}

let onUnauthorized: (() => void) | undefined

/** The session registers here so that any 401 returns the user to sign-in. */
export function setUnauthorizedHandler(handler: (() => void) | undefined): void {
  onUnauthorized = handler
}

/** For requests that cannot go through `api()` (XHR uploads): a 401 returns the user to sign-in too. */
export function reportUnauthorized(): void {
  onUnauthorized?.()
}

export function isErrorEnvelope(value: unknown): value is { error: { code: string; message: string } } {
  if (typeof value !== 'object' || value === null || !('error' in value)) return false
  const error = (value as { error: unknown }).error
  return typeof error === 'object' && error !== null && typeof (error as { code?: unknown }).code === 'string'
}

/** Sends a JSON request and returns the `data` of the `{ data }` envelope. */
export async function api<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, signal, keepSession = false, adminPassword } = options
  let response: Response
  try {
    response = await fetch(API_BASE + path, {
      method,
      credentials: 'same-origin',
      signal,
      headers: {
        ...(body === undefined ? {} : { 'Content-Type': 'application/json' }),
        ...(adminPassword === undefined ? {} : { 'X-Admin-Password': adminPassword }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError(0, 'NETWORK', 'network error')
  }

  const payload: unknown = await response.json().catch(() => undefined)
  if (!response.ok) {
    if (response.status === 401 && !keepSession && adminPassword === undefined) onUnauthorized?.()
    if (isErrorEnvelope(payload)) throw new ApiError(response.status, payload.error.code, payload.error.message)
    throw new ApiError(response.status, 'UNKNOWN', response.statusText)
  }
  if (typeof payload === 'object' && payload !== null && 'data' in payload) return (payload as { data: T }).data
  return undefined as T
}
