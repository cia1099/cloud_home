import { describe, expect, it } from 'vitest'
import { redirectTarget } from './auth/redirect'
import { backoffDelay } from './sse/backoff'
import { resolveTheme } from './theme/theme'

describe('resolveTheme', () => {
  it('follows the system only in automatic mode', () => {
    expect(resolveTheme('auto', true)).toBe('dark')
    expect(resolveTheme('auto', false)).toBe('light')
    expect(resolveTheme('light', true)).toBe('light')
    expect(resolveTheme('dark', false)).toBe('dark')
  })
})

describe('backoffDelay', () => {
  it('doubles from 1 s and caps at 10 s', () => {
    expect([0, 1, 2, 3, 4, 9].map(backoffDelay)).toEqual([1000, 2000, 4000, 8000, 10_000, 10_000])
  })
})

describe('redirectTarget', () => {
  it('returns the remembered local route and refuses anything else', () => {
    expect(redirectTarget({ from: '/files/abc?q=1' })).toBe('/files/abc?q=1')
    expect(redirectTarget({ from: '//evil.example' })).toBe('/files')
    expect(redirectTarget({ from: 'https://evil.example' })).toBe('/files')
    expect(redirectTarget(null)).toBe('/files')
  })
})
