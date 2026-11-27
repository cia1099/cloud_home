import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { rememberAppearance, resolveTheme, storedAppearance, type Appearance } from './theme'
import { ThemeContext } from './useTheme'

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [appearance, setAppearanceState] = useState<Appearance>(storedAppearance)

  // Automatic follows the system live while the page is open.
  useEffect(() => {
    const query = matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(appearance, query.matches)
    }
    apply()
    query.addEventListener('change', apply)
    return () => query.removeEventListener('change', apply)
  }, [appearance])

  const setAppearance = useCallback((next: Appearance) => {
    rememberAppearance(next)
    setAppearanceState(next)
  }, [])

  const value = useMemo(() => ({ appearance, setAppearance }), [appearance, setAppearance])
  return <ThemeContext value={value}>{children}</ThemeContext>
}
