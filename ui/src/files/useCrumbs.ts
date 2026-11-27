import { useEffect, useState } from 'react'
import { getFile } from '../api/files'
import { ancestorsOf, type Crumb } from './model'

/** The folders from the top down to `folderId`, resolved through `parent_id`. Empty for the root or while loading. */
export function useCrumbs(folderId: string | null): Crumb[] {
  const [resolved, setResolved] = useState<{ id: string; crumbs: Crumb[] } | null>(null)

  useEffect(() => {
    if (!folderId) return
    let cancelled = false
    getFile(folderId)
      .then((folder) => ancestorsOf(folder, getFile))
      .then((crumbs) => {
        if (!cancelled) setResolved({ id: folderId, crumbs })
      })
      .catch(() => undefined) // the listing shows the error; the path simply stays empty
    return () => {
      cancelled = true
    }
  }, [folderId])

  return folderId && resolved?.id === folderId ? resolved.crumbs : []
}
