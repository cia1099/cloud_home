import { api } from './client'
import type { User } from './auth'

type Provider = 'email' | 'phone'

export const updateName = (username: string) => api<User>('/me', { method: 'PATCH', body: { username } })

export const changePassword = (current_password: string, new_password: string) =>
  api<{ ok: true }>('/me/password', { method: 'POST', body: { current_password, new_password }, keepSession: true })

/** Sends a one-time code to the existing identifier; required before adding, changing or removing one. */
export const sendChangeCode = (provider: Provider) => api<void>('/me/identifier/challenge', { method: 'POST', body: { provider } })

export const setIdentifier = (provider: Provider, new_value: string, current_password: string, old_code: string) =>
  api<User>('/me/identifier', { method: 'PUT', body: { provider, new_value, current_password, old_code }, keepSession: true })

export const removeIdentifier = (provider: Provider, current_password: string, old_code: string) =>
  api<User>('/me/identifier', { method: 'DELETE', body: { provider, current_password, old_code }, keepSession: true })

/** Sends a code to an identifier of the account that is not verified yet. */
export const sendVerifyCode = (provider: Provider) => api<void>('/me/identifier/verify/request', { method: 'POST', body: { provider } })

export const verifyIdentifier = (provider: Provider, code: string) =>
  api<User>('/me/identifier/verify', { method: 'POST', body: { provider, code } })
