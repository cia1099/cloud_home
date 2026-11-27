import { useCallback, useRef, useState, type ChangeEvent } from 'react'
import { Icon } from '../components/Icon'
import { Menu, type MenuAnchor } from '../components/Menu'
import { useI18n } from '../i18n/useI18n'
import { fromInput } from './collect'
import type { Picked } from './model'

/** Toolbar button with the Upload Files / Upload Folder / New Folder menu and the hidden pickers behind it. */
export function UploadButton({ disabled, onPick, onNewFolder }: { disabled: boolean; onPick: (picked: Picked[]) => void; onNewFolder: () => void }) {
  const { t } = useI18n()
  const files = useRef<HTMLInputElement>(null)
  const folder = useRef<HTMLInputElement>(null)
  const [anchor, setAnchor] = useState<MenuAnchor | null>(null)
  const close = useCallback(() => setAnchor(null), [])

  const picked = (event: ChangeEvent<HTMLInputElement>) => {
    onPick(fromInput(event.target.files))
    event.target.value = '' // lets the same selection be chosen again
  }

  return (
    <>
      <button
        type="button"
        className="btn primary"
        disabled={disabled}
        aria-haspopup="menu"
        onClick={(event) => {
          const box = event.currentTarget.getBoundingClientRect()
          setAnchor({ x: box.right - 220, y: box.bottom + 6 })
        }}
      >
        <Icon name="upload" size={18} />
        <span className="btn-label">{t('upload.menu')}</span>
      </button>
      <input ref={files} type="file" multiple hidden tabIndex={-1} onChange={picked} />
      <input ref={(el) => { folder.current = el; el?.setAttribute('webkitdirectory', '') }} type="file" hidden tabIndex={-1} onChange={picked} />
      {anchor && (
        <Menu
          anchor={anchor}
          label={t('upload.menu')}
          onClose={close}
          items={[
            { label: t('upload.files'), icon: 'file', onSelect: () => files.current?.click() },
            { label: t('upload.folder'), icon: 'folder', onSelect: () => folder.current?.click() },
            { label: t('files.new_folder'), icon: 'folderPlus', separated: true, onSelect: onNewFolder },
          ]}
        />
      )}
    </>
  )
}
