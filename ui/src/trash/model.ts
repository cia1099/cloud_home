const DAY_MS = 86_400_000

const startOfDay = (time: number) => {
  const d = new Date(time)
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
}

/** Whole calendar days until `expiresAt` (local time). 0 means "today" and past dates stay 0. */
export function daysLeft(expiresAt: string, now: number): number {
  const expiry = Date.parse(expiresAt)
  if (Number.isNaN(expiry)) return 0
  return Math.max(0, Math.round((startOfDay(expiry) - startOfDay(now)) / DAY_MS))
}

/** Amber warning from one day left (design/app-spec.md section 7). */
export const isUrgent = (days: number) => days <= 1
