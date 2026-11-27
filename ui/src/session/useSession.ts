import { createContext, useContext } from 'react'
import type { OAuthProvider } from '../api/oauth'
import type { Credentials, Registration, User } from '../api/auth'

export type Session =
  | { phase: 'loading' }
  | { phase: 'error'; error: unknown }
  | { phase: 'anon'; expired: boolean }
  | { phase: 'authed'; user: User }

export interface SessionApi {
  session: Session
  signIn: (credentials: Credentials) => Promise<void>
  /** Resolves to `pending_approval` when the account waits for the administrator (no session is created). */
  signUp: (registration: Registration) => Promise<'signed_in' | 'pending_approval'>
  /** Finishes a third-party sign-in; resolves like `signUp` (`pending_approval` creates no session). */
  signInWithOAuth: (provider: OAuthProvider, code: string, redirectUri: string) => Promise<'signed_in' | 'pending_approval'>
  signOut: () => Promise<void>
  /** Replaces the signed-in user after a profile change (the API returns the updated user). */
  setUser: (user: User) => void
  /** Re-runs `GET /auth/me` after a failed start (for example while offline). */
  retry: () => void
}

export const SessionContext = createContext<SessionApi | null>(null)

export function useSession(): SessionApi {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession outside SessionProvider')
  return value
}
