import type { Identity, User } from '../api/auth'

export type Provider = 'email' | 'phone'

const NAME_MIN = 3
const NAME_MAX = 64
const PASSWORD_MIN = 8
const CODE_LENGTH = 6
export const RESEND_SECONDS = 60

/** Mirrors the server: 3 to 64 characters after trimming (code points, like Rust's `chars().count()`). */
export function checkName(raw: string): 'ok' | 'short' | 'long' {
  const length = [...raw.trim()].length
  return length < NAME_MIN ? 'short' : length > NAME_MAX ? 'long' : 'ok'
}

/** Lower-cased address, or `null` when it has no local part or no dotted domain (the server's rule). */
export function normalizeEmail(raw: string): string | null {
  const email = raw.trim().toLowerCase()
  const at = email.indexOf('@')
  const domain = email.slice(at + 1)
  if (at < 1 || !domain.includes('.') || /\s/.test(email)) return null
  return email
}

/** The server counts bytes, not characters. */
export function isPasswordLongEnough(password: string): boolean {
  return new TextEncoder().encode(password).length >= PASSWORD_MIN
}

export function isCodeComplete(code: string): boolean {
  return code.length === CODE_LENGTH
}

/** Keeps digits only, so a pasted "123 456" works. */
export function cleanCode(raw: string): string {
  return raw.replace(/\D/g, '').slice(0, CODE_LENGTH)
}

export function identityOf(user: User, provider: Provider): Identity | undefined {
  return user.identities.find((i) => i.provider === provider)
}

/** An account keeps at least one way to sign in; only an existing identifier can be removed. */
export function canRemove(user: User, provider: Provider): boolean {
  return user.identities.filter((i) => i.provider === 'email' || i.provider === 'phone').length > 1 && identityOf(user, provider) !== undefined
}

/**
 * Where the server sends the code for add/change/remove (`POST /me/identifier/challenge`): to the
 * identifier of the same kind, or, when adding the first of a kind, to the other one.
 */
export function challengeTarget(user: User, provider: Provider): string | null {
  const other: Provider = provider === 'email' ? 'phone' : 'email'
  return (identityOf(user, provider) ?? identityOf(user, other))?.identifier ?? null
}

/** Whole seconds until a new code may be requested. */
export function secondsLeft(sentAt: number, now: number): number {
  return Math.max(0, RESEND_SECONDS - Math.floor((now - sentAt) / 1000))
}
