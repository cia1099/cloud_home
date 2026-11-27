import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { ToastContext, type ToastOptions } from './useToast'

const DURATION_MS = 4500

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toast, setToast] = useState<ToastOptions | null>(null)

  const show = useCallback((next: ToastOptions) => setToast(next), [])

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(null), DURATION_MS)
    return () => window.clearTimeout(timer)
  }, [toast])

  return (
    <ToastContext value={show}>
      {children}
      <div className="toast-region" role="status" aria-live="polite">
        {toast && (
          <div className="toast">
            <span>{toast.text}</span>
            {toast.action && (
              <button
                type="button"
                className="toast-action"
                onClick={() => {
                  setToast(null)
                  toast.action?.run()
                }}
              >
                {toast.action.label}
              </button>
            )}
          </div>
        )}
      </div>
    </ToastContext>
  )
}
