import { createContext, useContext } from 'react'
import type { Appearance } from './theme'

export interface Theme {
  appearance: Appearance
  setAppearance: (appearance: Appearance) => void
}

export const ThemeContext = createContext<Theme | null>(null)

export function useTheme(): Theme {
  const value = useContext(ThemeContext)
  if (!value) throw new Error('useTheme outside ThemeProvider')
  return value
}
