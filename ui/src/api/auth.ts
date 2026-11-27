import { api } from './client'

export interface User {
  id: string
  username: string
  email: string | null
  phone: string | null
}

export interface Credentials {
  identifier: string
  password: string
}

export interface Registration {
  username: string
  password: string
  email?: string
  phone?: string
}

/** `201` signs the user in; `202` means the account waits for the administrator. */
export type RegisterResult = { status: 'signed_in'; user: User } | { status: 'pending_approval' }

export const fetchMe = (signal?: AbortSignal) => api<User>('/auth/me', { signal })

export async function login(credentials: Credentials): Promise<User> {
  const result = await api<{ user: User }>('/auth/login', { method: 'POST', body: credentials })
  return result.user
}

export async function register(registration: Registration): Promise<RegisterResult> {
  const result = await api<{ status: 'pending_approval' } | { user: User }>('/auth/register', {
    method: 'POST',
    body: registration,
  })
  return 'user' in result ? { status: 'signed_in', user: result.user } : { status: 'pending_approval' }
}

export const logout = () => api<void>('/auth/logout', { method: 'POST' })
