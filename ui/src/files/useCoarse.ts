import { useSyncExternalStore } from 'react'

const QUERY = '(pointer: coarse), (max-width: 699px)'

function subscribe(callback: () => void): () => void {
  const list = window.matchMedia(QUERY)
  list.addEventListener('change', callback)
  return () => list.removeEventListener('change', callback)
}

/** Touch behaviour (tap opens, tick selects) on coarse pointers and on phone-width screens. */
export function useCoarse(): boolean {
  return useSyncExternalStore(subscribe, () => window.matchMedia(QUERY).matches)
}
