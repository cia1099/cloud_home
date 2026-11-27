import { useState } from 'react'
import { Dialog } from './Dialog'
import { useI18n } from '../i18n/useI18n'

/** The alert for irreversible actions (Delete Forever, Empty Trash, Stop Sharing). Closes itself after `onConfirm`. */
export function ConfirmDialog({
  title,
  text,
  confirmLabel,
  onConfirm,
  onClose,
}: {
  title: string
  text: string
  confirmLabel: string
  onConfirm: () => Promise<void>
  onClose: () => void
}) {
  const { t } = useI18n()
  const [busy, setBusy] = useState(false)
  return (
    <Dialog title={title} alert onClose={onClose}>
      <p className="hint">{text}</p>
      <div className="row-end">
        <button type="button" className="btn plain" onClick={onClose}>
          {t('files.cancel')}
        </button>
        <button
          type="button"
          className="btn danger"
          disabled={busy}
          onClick={() => {
            setBusy(true)
            void onConfirm().finally(onClose)
          }}
        >
          {confirmLabel}
        </button>
      </div>
    </Dialog>
  )
}
