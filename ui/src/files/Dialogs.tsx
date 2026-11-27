import { useEffect, useRef, useState, type FormEvent } from 'react'
import type { ArchiveFormat } from '../api/download'
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

/** Folders and multiple items leave as one archive; the person picks ZIP (default) or tar.gz. */
export function DownloadDialog({ count, onDownload, onClose }: { count: number; onDownload: (format: ArchiveFormat) => Promise<void>; onClose: () => void }) {
  const { t, tn } = useI18n()
  const [format, setFormat] = useState<ArchiveFormat>('zip')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setBusy(true)
    try {
      await onDownload(format)
      onClose()
    } catch (cause) {
      // Dismissing the browser's save dialog is not a failure.
      if (cause instanceof DOMException && cause.name === 'AbortError') onClose()
      else {
        setError(t(errorKey(cause)))
        setBusy(false)
      }
    }
  }

  return (
    <Dialog title={t('download.title')} onClose={onClose}>
      <form onSubmit={(event) => void submit(event)} className="dialog-form">
        <p className="small">{tn('download.summary', count)}</p>
        <fieldset className="choice">
          <legend className="label">{t('download.format')}</legend>
          {(['zip', 'tar.gz'] as const).map((value) => (
            <label key={value}>
              <input type="radio" name="download-format" checked={format === value} onChange={() => setFormat(value)} />
              <span>
                {t(value === 'zip' ? 'download.zip' : 'download.targz')}
                <span className="small"> {t(value === 'zip' ? 'download.zip.hint' : 'download.targz.hint')}</span>
              </span>
            </label>
          ))}
        </fieldset>
        <ErrorLine message={error} />
        <div className="row-end">
          <button type="button" className="btn plain" onClick={onClose}>
            {t('files.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={busy}>
            <Icon name="download" size={18} />
            {t('download.action')}
          </button>
        </div>
      </form>
    </Dialog>
  )
}
