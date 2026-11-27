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
}

let onUnauthorized: (() => void) | undefined

/** The session registers here so that any 401 returns the user to sign-in. */
export function setUnauthorizedHandler(handler: (() => void) | undefined): void {
  onUnauthorized = handler
}

function isErrorEnvelope(value: unknown): value is { error: { code: string; message: string } } {
  if (typeof value !== 'object' || value === null || !('error' in value)) return false
  const error = (value as { error: unknown }).error
  return typeof error === 'object' && error !== null && typeof (error as { code?: unknown }).code === 'string'
}

/** Sends a JSON request and returns the `data` of the `{ data }` envelope. */
export async function api<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, signal } = options
  let response: Response
  try {
    response = await fetch(API_BASE + path, {
      method,
      credentials: 'same-origin',
      signal,
      headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError(0, 'NETWORK', 'network error')
  }

  const payload: unknown = await response.json().catch(() => undefined)
  if (!response.ok) {
    if (response.status === 401) onUnauthorized?.()
    if (isErrorEnvelope(payload)) throw new ApiError(response.status, payload.error.code, payload.error.message)
    throw new ApiError(response.status, 'UNKNOWN', response.statusText)
  }
  if (typeof payload === 'object' && payload !== null && 'data' in payload) return (payload as { data: T }).data
  return undefined as T
}
