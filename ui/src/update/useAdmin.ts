import { createContext, useContext } from 'react'
import type { UpdateStatus } from '../api/update'

/**
 * checking: probing. unavailable: not for this caller (no admin password on the box, or not on the
 * local network). locked: available, password needed. unlocked: the password was accepted.
 */
export type AdminAccess = { state: 'checking' | 'unavailable' | 'locked' } | { state: 'unlocked'; password: string; first: UpdateStatus }

export interface AdminApi {
  access: AdminAccess
  /** Verifies the password with the box; rejects with the `ApiError` when it is wrong. */
  unlock: (password: string) => Promise<void>
}

export const AdminContext = createContext<AdminApi | null>(null)

export function useAdmin(): AdminApi {
  const value = useContext(AdminContext)
  if (!value) throw new Error('useAdmin outside AdminProvider')
  return value
}
