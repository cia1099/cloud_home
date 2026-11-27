import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import {
  fetchMe,
  login as apiLogin,
  logout as apiLogout,
  register as apiRegister,
  type Credentials,
  type Registration,
  type User,
} from '../api/auth'
import { oauthLogin, type OAuthProvider } from '../api/oauth'
import { ApiError, setUnauthorizedHandler } from '../api/client'
import { SessionContext, type Session } from './useSession'

export function SessionProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session>({ phase: 'loading' })
  const [attempt, setAttempt] = useState(0)
  const authed = useRef(false)

  useEffect(() => {
    authed.current = session.phase === 'authed'
  }, [session.phase])

  // `GET /auth/me` on load (app-spec section 8). 401 simply means "not signed in".
  useEffect(() => {
    const controller = new AbortController()
    fetchMe(controller.signal).then(
      (user) => setSession({ phase: 'authed', user }),
      (error: unknown) => {
        if (controller.signal.aborted) return
        setSession(error instanceof ApiError && error.status === 401 ? { phase: 'anon', expired: false } : { phase: 'error', error })
      },
    )
    return () => controller.abort()
  }, [attempt])

  // A 401 while signed in means the session ended: back to sign-in, keeping the route (see RequireAuth).
  useEffect(() => {
    setUnauthorizedHandler(() => {
      if (authed.current) setSession({ phase: 'anon', expired: true })
    })
    return () => setUnauthorizedHandler(undefined)
  }, [])

  const signIn = useCallback(async (credentials: Credentials) => {
    const user = await apiLogin(credentials)
    setSession({ phase: 'authed', user })
  }, [])

  const signUp = useCallback(async (registration: Registration) => {
    const result = await apiRegister(registration)
    if (result.status === 'signed_in') setSession({ phase: 'authed', user: result.user })
    return result.status
  }, [])

  const signInWithOAuth = useCallback(async (provider: OAuthProvider, code: string, redirectUri: string) => {
    const result = await oauthLogin(provider, code, redirectUri)
    if (result.status === 'signed_in') setSession({ phase: 'authed', user: result.user })
    return result.status
  }, [])

  const signOut = useCallback(async () => {
    // The cookie is cleared by the server; even if the request fails the user wants out.
    await apiLogout().catch(() => undefined)
    setSession({ phase: 'anon', expired: false })
  }, [])

  const setUser = useCallback((user: User) => setSession({ phase: 'authed', user }), [])

  const retry = useCallback(() => {
    setSession({ phase: 'loading' })
    setAttempt((n) => n + 1)
  }, [])

  const value = useMemo(() => ({ session, signIn, signUp, signInWithOAuth, signOut, setUser, retry }), [session, signIn, signUp, signInWithOAuth, signOut, setUser, retry])
  return <SessionContext value={value}>{children}</SessionContext>
}
