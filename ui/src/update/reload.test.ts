import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { clearReloadGuard, reloadAlreadyTried, reloadForUpdate, takeUpdatedVersion } from './reload'

function memoryStorage(): Storage {
  const map = new Map<string, string>()
  return {
    get length() {
      return map.size
    },
    clear: () => map.clear(),
    getItem: (key) => map.get(key) ?? null,
    key: (index) => [...map.keys()][index] ?? null,
    removeItem: (key) => void map.delete(key),
    setItem: (key, value) => void map.set(key, value),
  }
}

describe('reload guard', () => {
  const reload = vi.fn()
  beforeEach(() => {
    vi.stubGlobal('sessionStorage', memoryStorage())
    vi.stubGlobal('window', { location: { reload } })
    reload.mockClear()
  })
  afterEach(() => vi.unstubAllGlobals())

  it('remembers the version it reloaded for, announces it once, and blocks a second reload for it', () => {
    expect(reloadAlreadyTried('0.4.0')).toBe(false)
    reloadForUpdate('0.4.0')
    expect(reload).toHaveBeenCalledOnce()
    expect(reloadAlreadyTried('0.4.0')).toBe(true)
    expect(reloadAlreadyTried('0.5.0')).toBe(false)
    expect(takeUpdatedVersion()).toBe('0.4.0')
    expect(takeUpdatedVersion()).toBeNull()
  })

  it('a reload for a missing asset blocks every further reload until the versions agree again', () => {
    reloadForUpdate(null)
    expect(reloadAlreadyTried('0.4.0')).toBe(true)
    expect(takeUpdatedVersion()).toBeNull()
    clearReloadGuard()
    expect(reloadAlreadyTried('0.4.0')).toBe(false)
  })
})
