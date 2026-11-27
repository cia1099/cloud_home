import { useCallback, useState } from 'react'
import { DEFAULT_SORT, type Sort } from './model'

export type View = 'grid' | 'list'

/** View mode and sort order are per-browser preferences (design/app-spec.md section 1). */
function usePreference<T>(storageKey: string, fallback: T, parse: (raw: string) => T | undefined): [T, (value: T) => void] {
  const [value, setValue] = useState<T>(() => {
    const raw = localStorage.getItem(storageKey)
    return (raw === null ? undefined : parse(raw)) ?? fallback
  })
  const update = useCallback(
    (next: T) => {
      localStorage.setItem(storageKey, typeof next === 'string' ? next : JSON.stringify(next))
      setValue(next)
    },
    [storageKey],
  )
  return [value, update]
}

export const useView = () =>
  usePreference<View>('files.view', 'grid', (raw) => (raw === 'grid' || raw === 'list' ? raw : undefined))

export const useSort = () =>
  usePreference<Sort>('files.sort', DEFAULT_SORT, (raw) => {
    try {
      const value: unknown = JSON.parse(raw)
      if (typeof value !== 'object' || value === null) return undefined
      const { key, dir } = value as Partial<Sort>
      return (key === 'name' || key === 'modified' || key === 'size') && (dir === 'asc' || dir === 'desc') ? { key, dir } : undefined
    } catch {
      return undefined
    }
  })
