export const EXPIRY_CHOICES = ['1d', '7d', '30d', 'never'] as const
export type Expiry = (typeof EXPIRY_CHOICES)[number]

const DAYS: Record<Exclude<Expiry, 'never'>, number> = { '1d': 1, '7d': 7, '30d': 30 }

/** RFC 3339 instant for the API, or null for a link that never expires. */
export function expiryToIso(choice: Expiry, now: number): string | null {
  return choice === 'never' ? null : new Date(now + DAYS[choice] * 86_400_000).toISOString()
}

export function isExpired(expiresAt: string | null, now: number): boolean {
  return expiresAt !== null && Date.parse(expiresAt) <= now
}

/** The page address a visitor opens; the app itself serves `/s/:token`. */
export const shareUrl = (origin: string, token: string) => `${origin}/s/${encodeURIComponent(token)}`

/** Amber expiry chip on a received share when this many days or fewer remain. */
const EXPIRY_WARN_MS = 3 * 86_400_000
export const expiresSoon = (expiresAt: string | null, now: number) => expiresAt !== null && Date.parse(expiresAt) - now <= EXPIRY_WARN_MS
