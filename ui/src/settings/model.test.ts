import { describe, expect, it } from 'vitest'
import type { Identity, User } from '../api/auth'
import { byRegion, checkPhone, filterCountries, formatPhone, listCountries, regionName, splitPhone } from './countries'
import { canRemove, challengeTarget, checkName, cleanCode, isCodeComplete, isPasswordLongEnough, normalizeEmail, secondsLeft } from './model'

const id = (provider: string, identifier: string, managed_by: string | null = null): Identity => ({ provider, identifier, verified: true, managed_by })
const user = (...identities: Identity[]): User => ({ id: 'u', username: 'Ann', email: null, phone: null, has_password: true, identities })

describe('checkName', () => {
  it('follows the server limits after trimming, counting characters', () => {
    expect(checkName('  ab ')).toBe('short')
    expect(checkName('abc')).toBe('ok')
    expect(checkName('小明明')).toBe('ok')
    expect(checkName('a'.repeat(64))).toBe('ok')
    expect(checkName('a'.repeat(65))).toBe('long')
  })
})

describe('normalizeEmail', () => {
  it('lowercases and trims, needs a local part and a dotted domain', () => {
    expect(normalizeEmail('  Dad@GMail.com ')).toBe('dad@gmail.com')
    expect(normalizeEmail('dad@localhost')).toBeNull()
    expect(normalizeEmail('@gmail.com')).toBeNull()
    expect(normalizeEmail('dad')).toBeNull()
    expect(normalizeEmail('da d@x.com')).toBeNull()
  })
})

describe('password and code', () => {
  it('counts bytes like the server', () => {
    expect(isPasswordLongEnough('1234567')).toBe(false)
    expect(isPasswordLongEnough('12345678')).toBe(true)
    expect(isPasswordLongEnough('密码密')).toBe(true)
  })
  it('keeps only six digits', () => {
    expect(cleanCode('123 456')).toBe('123456')
    expect(cleanCode('12a34b56789')).toBe('123456')
    expect(isCodeComplete('12345')).toBe(false)
    expect(isCodeComplete('123456')).toBe(true)
  })
  it('counts the resend countdown down to zero', () => {
    expect(secondsLeft(0, 0)).toBe(60)
    expect(secondsLeft(0, 15_500)).toBe(45)
    expect(secondsLeft(0, 90_000)).toBe(0)
  })
})

describe('identifiers', () => {
  it('always keeps one way to sign in', () => {
    const both = user(id('email', 'a@b.co'), id('phone', '+886912345678'))
    expect(canRemove(both, 'email')).toBe(true)
    const onlyEmail = user(id('email', 'a@b.co'))
    expect(canRemove(onlyEmail, 'email')).toBe(false)
    expect(canRemove(onlyEmail, 'phone')).toBe(false)
    expect(canRemove(user(id('email', 'a@b.co'), id('google', 'g')), 'email')).toBe(false)
  })
  it('sends the change code to the same kind, or to the other one when adding', () => {
    const both = user(id('email', 'a@b.co'), id('phone', '+886912345678'))
    expect(challengeTarget(both, 'phone')).toBe('+886912345678')
    expect(challengeTarget(user(id('email', 'a@b.co')), 'phone')).toBe('a@b.co')
    expect(challengeTarget(user(), 'phone')).toBeNull()
  })
})

describe('phone numbers', () => {
  it('builds E.164 and drops the trunk zero', () => {
    expect(checkPhone(byRegion('TW'), '0912 345 678')).toEqual({ ok: true, e164: '+886912345678' })
    expect(checkPhone(byRegion('CN'), '138 0013 8000')).toEqual({ ok: true, e164: '+8613800138000' })
    expect(checkPhone(byRegion('US'), '(555) 013-4000')).toEqual({ ok: true, e164: '+15550134000' })
  })
  it('keeps the leading zero where it belongs to the number', () => {
    expect(checkPhone(byRegion('IT'), '0212345678')).toEqual({ ok: true, e164: '+390212345678' })
  })
  it('rejects wrong lengths and stray characters', () => {
    expect(checkPhone(byRegion('TW'), '91234')).toEqual({ ok: false })
    expect(checkPhone(byRegion('US'), '555 013 40001')).toEqual({ ok: false })
    expect(checkPhone(byRegion('US'), '555abc0134')).toEqual({ ok: false })
    expect(checkPhone(byRegion('FR'), '')).toEqual({ ok: false })
  })
  it('splits E.164 numbers by calling code and formats them', () => {
    expect(splitPhone('+886912345678')?.country.region).toBe('TW')
    expect(splitPhone('+15550134000')?.country.region).toBe('US')
    expect(splitPhone('+78005553535')?.country.region).toBe('RU')
    expect(splitPhone('0912345678')).toBeNull()
    expect(formatPhone('+8613800138000')).toBe('+86 13800138000')
  })
})

describe('country list', () => {
  it('uses the glossary names for Chinese and finds entries by name, ISO code or dial code', () => {
    expect(regionName('zh-Hant', 'TW')).toBe('臺灣')
    expect(regionName('zh-Hans', 'CN')).toBe('中国大陆')
    const all = listCountries('en')
    expect(filterCountries(all, 'japan').map((c) => c.region)).toContain('JP')
    expect(filterCountries(all, 'tw').map((c) => c.region)).toContain('TW')
    expect(filterCountries(all, '+886').map((c) => c.region)).toEqual(['TW'])
    expect(filterCountries(all, 'zzzz')).toEqual([])
    expect(filterCountries(all, '')).toHaveLength(all.length)
  })
})
