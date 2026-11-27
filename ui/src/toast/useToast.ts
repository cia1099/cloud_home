import { createContext, useContext } from 'react'

export interface ToastOptions {
  text: string
  /** One optional button, for example Undo. */
  action?: { label: string; run: () => void }
}

export const ToastContext = createContext<((toast: ToastOptions) => void) | null>(null)

/** Shows one toast at a time for 4.5 s (design/app-spec.md section 3). */
export function useToast(): (toast: ToastOptions) => void {
  const value = useContext(ToastContext)
  if (!value) throw new Error('useToast outside ToastProvider')
  return value
}
