const UPDATED_KEY = 'cloud_home.updated_to'
const TRIED_KEY = 'cloud_home.reload_tried'

/** Marker for a reload caused by a missing hashed asset, where no new version number is known. */
const ANY = '*'

function remember(key: string, value: string): void {
  try {
    sessionStorage.setItem(key, value)
  } catch {
    // Private mode without storage: the reload still happens, only the "Updated to" toast and the loop guard are lost.
  }
}

function recall(key: string): string | null {
  try {
    return sessionStorage.getItem(key)
  } catch {
    return null
  }
}

/** Reloads the page to pick up the new bundle; `version` (when known) is announced after the reload. */
export function reloadForUpdate(version: string | null): void {
  if (version) remember(UPDATED_KEY, version)
  remember(TRIED_KEY, version ?? ANY)
  window.location.reload()
}

/** True when this tab already reloaded for `version` (or for a missing asset) and is still out of date. */
export function reloadAlreadyTried(version: string | null): boolean {
  const tried = recall(TRIED_KEY)
  return tried !== null && (tried === (version ?? ANY) || tried === ANY)
}

/** Forget the loop guard once the bundle and the box agree again. */
export function clearReloadGuard(): void {
  try {
    sessionStorage.removeItem(TRIED_KEY)
  } catch {
    // Nothing stored, nothing to clear.
  }
}

/** The version this tab just reloaded into, once. */
export function takeUpdatedVersion(): string | null {
  const version = recall(UPDATED_KEY)
  if (version !== null) {
    try {
      sessionStorage.removeItem(UPDATED_KEY)
    } catch {
      // Same as above.
    }
  }
  return version
}
