import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { fetchUpdateStatus, probeAdmin } from '../api/update'
import { AdminContext, type AdminAccess, type AdminApi } from './useAdmin'

/**
 * The box has one shared administrator password, not per-user admin rights (spec/backend.md). The
 * Administration entry is therefore shown to whoever the box would let in at all, and the password
 * is asked for on the page. It lives in memory only: never in storage, never in the URL.
 */
export function AdminProvider({ children }: { children: ReactNode }) {
  const [access, setAccess] = useState<AdminAccess>({ state: 'checking' })

  useEffect(() => {
    const controller = new AbortController()
    void probeAdmin(controller.signal).then((available) => {
      if (!controller.signal.aborted) setAccess({ state: available ? 'locked' : 'unavailable' })
    })
    return () => controller.abort()
  }, [])

  const unlock = useCallback(async (password: string) => {
    const first = await fetchUpdateStatus(password)
    setAccess({ state: 'unlocked', password, first })
  }, [])

  const value = useMemo<AdminApi>(() => ({ access, unlock }), [access, unlock])
  return <AdminContext value={value}>{children}</AdminContext>
}
