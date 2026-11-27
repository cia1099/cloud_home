import { useEffect, useId, useRef, type ReactNode } from 'react'

/** Modal dialog on the native `<dialog>` element: focus trap, Esc and focus return come from the browser. */
export function Dialog({ title, onClose, alert = false, children }: { title: string; onClose: () => void; alert?: boolean; children: ReactNode }) {
  const ref = useRef<HTMLDialogElement>(null)
  const titleId = useId()

  useEffect(() => {
    const dialog = ref.current
    if (!dialog) return
    dialog.showModal()
    return () => dialog.close()
  }, [])

  return (
    <dialog
      ref={ref}
      className="dialog"
      role={alert ? 'alertdialog' : undefined}
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault()
        onClose()
      }}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose()
      }}
    >
      <h2 id={titleId}>{title}</h2>
      {children}
    </dialog>
  )
}
