import { useEffect, useRef, useState, type FormEvent } from 'react'
import { listFolder, type FileItem } from '../api/files'
import { errorKey } from '../api/errors'
import { Dialog } from '../components/Dialog'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'

function ErrorLine({ message }: { message: string | null }) {
  return message ? (
    <p className="hint err" role="alert">
      <Icon name="alert" size={14} />
      {message}
    </p>
  ) : null
}

/** Asks for a folder name. The server renames on a clash (`Name(1)`), so the only failures are real errors. */
export function NewFolderDialog({ onCreate, onClose }: { onCreate: (name: string) => Promise<void>; onClose: () => void }) {
  const { t } = useI18n()
  const input = useRef<HTMLInputElement>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => input.current?.select(), [])

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const name = input.current?.value.trim() ?? ''
    if (!name) return
    setBusy(true)
    try {
      await onCreate(name)
      onClose()
    } catch (cause) {
      setError(t(errorKey(cause)))
      setBusy(false)
    }
  }

  return (
    <Dialog title={t('files.new_folder')} onClose={onClose}>
      <form onSubmit={(event) => void submit(event)} className="dialog-form">
        <div className="field">
          <label htmlFor="new-folder-name">{t('files.folder_name')}</label>
          <input
            id="new-folder-name"
            ref={input}
            className="input"
            defaultValue={t('files.untitled_folder')}
            spellCheck={false}
            autoComplete="off"
            aria-invalid={error !== null}
          />
        </div>
        <ErrorLine message={error} />
        <div className="row-end">
          <button type="button" className="btn plain" onClick={onClose}>
            {t('files.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={busy}>
            {t('files.create')}
          </button>
        </div>
      </form>
    </Dialog>
  )
}

interface Level {
  id: string | null
  name: string
}

/** Folder picker for Move. Folders being moved are not listed, so an item can never be moved into itself. */
export function MoveDialog({
  moving,
  onMove,
  onClose,
}: {
  moving: readonly FileItem[]
  onMove: (targetId: string | null) => Promise<void>
  onClose: () => void
}) {
  const { t } = useI18n()
  const [path, setPath] = useState<Level[]>([{ id: null, name: t('nav.files') }])
  const [folders, setFolders] = useState<{ key: string; items?: FileItem[]; error?: unknown }>({ key: '' })
  const [tick, setTick] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const here = path[path.length - 1]
  const key = here.id ?? 'root'
  const movingIds = new Set(moving.map((item) => item.id))
  const alreadyHere = moving.every((item) => item.parent_id === here.id)

  useEffect(() => {
    const controller = new AbortController()
    listFolder(here.id, controller.signal).then(
      (items) => setFolders({ key, items: items.filter((item) => item.file_type === 'folder') }),
      (cause: unknown) => {
        if (!(cause instanceof DOMException)) setFolders({ key, error: cause })
      },
    )
    return () => controller.abort()
  }, [here.id, key, tick])

  const loaded = folders.key === key
  const submit = async () => {
    setBusy(true)
    setError(null)
    try {
      await onMove(here.id)
      onClose()
    } catch (cause) {
      setError(t(errorKey(cause)))
      setBusy(false)
    }
  }

  return (
    <Dialog title={t('files.move.title')} onClose={onClose}>
      <div className="picker-head">
        <button
          type="button"
          className="ibtn"
          aria-label={t('files.back')}
          disabled={path.length === 1}
          onClick={() => setPath((p) => p.slice(0, -1))}
        >
          <Icon name="chevL" size={20} />
        </button>
        <strong>{here.name}</strong>
      </div>
      <div className="picker" aria-busy={!loaded}>
        {!loaded && <p className="small pad-s">{t('loading')}</p>}
        {loaded && folders.error !== undefined && (
          <div className="pad-s">
            <p className="hint err">{t(errorKey(folders.error))}</p>
            <button type="button" className="btn text" onClick={() => setTick((n) => n + 1)}>
              {t('retry')}
            </button>
          </div>
        )}
        {loaded && folders.items?.filter((item) => !movingIds.has(item.id)).length === 0 && <p className="small pad-s">{t('files.move.empty')}</p>}
        {loaded &&
          folders.items
            ?.filter((item) => !movingIds.has(item.id))
            .map((item) => (
              <button key={item.id} type="button" className="picker-row" onClick={() => setPath((p) => [...p, { id: item.id, name: item.name }])}>
                <Icon name="folder" size={22} className="folder-glyph" />
                <span className="picker-name">{item.name}</span>
                <Icon name="chevR" size={16} />
              </button>
            ))}
      </div>
      <ErrorLine message={error} />
      <div className="row-end">
        <button type="button" className="btn plain" onClick={onClose}>
          {t('files.cancel')}
        </button>
        <button type="button" className="btn primary" disabled={busy || alreadyHere} onClick={() => void submit()}>
          {t('files.move.here')}
        </button>
      </div>
    </Dialog>
  )
}
