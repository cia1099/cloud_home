/** How long without input counts as idle for an automatic reload (design/app-spec.md section 9). */
export const IDLE_RELOAD_MS = 60_000

/** True when the box runs a different product version than the one this tab was built from. */
export const isStale = (build: string, server: string | null): boolean => server !== null && server !== '' && server !== build

export interface ReloadInput {
  /** The server's version differs from the bundle, or a hashed asset went missing. */
  stale: boolean
  /** An upload or an archive download is running in this tab. */
  busy: boolean
  /** Milliseconds since the last keyboard, pointer or touch input. */
  idleMs: number
  /** The user chose Later for this version. */
  dismissed: boolean
  /** A reload for this version already happened and did not help (stale cache): never loop. */
  alreadyTried: boolean
}

/**
 * none: nothing to show. prompt: the reload banner. wait: "will reload when transfers finish",
 * no reload. reload: reload now (idle, nothing running).
 */
export type ReloadDecision = 'none' | 'prompt' | 'wait' | 'reload'

export function reloadDecision({ stale, busy, idleMs, dismissed, alreadyTried }: ReloadInput): ReloadDecision {
  if (!stale || dismissed) return 'none'
  if (busy) return 'wait'
  if (alreadyTried) return 'prompt'
  return idleMs >= IDLE_RELOAD_MS ? 'reload' : 'prompt'
}

/** Only http(s) links from the box are opened; anything else is not a release page. */
export function safeNotesUrl(url: string): string | null {
  try {
    const parsed = new URL(url)
    return parsed.protocol === 'https:' || parsed.protocol === 'http:' ? parsed.href : null
  } catch {
    return null
  }
}
