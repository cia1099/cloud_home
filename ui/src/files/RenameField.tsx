import { useEffect, useRef, type KeyboardEvent } from 'react'
import type { FileItem } from '../api/files'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { splitName } from './model'

/**
 * Inline rename in the name cell. Starts with only the base name selected so typing keeps the
 * extension (owner decision, design/app-spec.md section 12). Enter or leaving the field saves, Esc cancels.
 */
export function RenameField({
  item,
  error,
  onCommit,
  onCancel,
}: {
  item: FileItem
  error: string | null
  onCommit: (name: string) => void
  onCancel: () => void
}) {
  const { t } = useI18n()
  const input = useRef<HTMLInputElement>(null)
  const finished = useRef(false)

  useEffect(() => {
    const field = input.current
    if (!field) return
    field.focus()
    field.setSelectionRange(0, splitName(item).base.length)
  }, [item])

  const commit = () => {
    if (finished.current) return
    finished.current = true
    onCommit(input.current?.value ?? item.name)
  }
  const cancel = () => {
    if (finished.current) return
    finished.current = true
    onCancel()
  }
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    event.stopPropagation()
    if (event.key === 'Enter' && !event.nativeEvent.isComposing) {
      event.preventDefault()
      commit()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      cancel()
    }
  }

  return (
    <span className="rename">
      <input
        ref={input}
        className="rename-input"
        defaultValue={item.name}
        aria-label={t('files.rename.label')}
        aria-invalid={error !== null}
        aria-describedby={error ? 'rename-error' : undefined}
        spellCheck={false}
        autoComplete="off"
        onKeyDown={onKeyDown}
        onClick={(event) => event.stopPropagation()}
        onDoubleClick={(event) => event.stopPropagation()}
        onChange={() => {
          // Typing after a failed attempt starts a fresh attempt.
          finished.current = false
        }}
        onBlur={() => (error ? onCancel() : commit())}
      />
      {error && (
        <span id="rename-error" className="hint err" role="alert">
          <Icon name="alert" size={14} />
          {error}
        </span>
      )}
    </span>
  )
}
