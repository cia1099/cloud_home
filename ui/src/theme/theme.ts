export const APPEARANCES = ['auto', 'light', 'dark'] as const
export type Appearance = (typeof APPEARANCES)[number]

const STORAGE_KEY = 'theme'

export function isAppearance(value: unknown): value is Appearance {
  return APPEARANCES.includes(value as Appearance)
}

/** Keep in sync with the inline script in index.html that runs before first paint. */
export function resolveTheme(appearance: Appearance, systemDark: boolean): 'light' | 'dark' {
  if (appearance === 'auto') return systemDark ? 'dark' : 'light'
  return appearance
}

export function storedAppearance(): Appearance {
  const stored = localStorage.getItem(STORAGE_KEY)
  return isAppearance(stored) ? stored : 'auto'
}

export function rememberAppearance(appearance: Appearance): void {
  localStorage.setItem(STORAGE_KEY, appearance)
}
