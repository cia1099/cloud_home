import { useEffect, useState } from 'react'
import { Dialog } from '../components/Dialog'
import { Icon } from '../components/Icon'
import { INTL_TAGS } from '../i18n/locale'
import { useI18n } from '../i18n/useI18n'
import { useOnline } from '../shell/useOnline'
import { durationParts, isActive, overall, remainingSeconds, toRows, wasRenamed, type Row } from './model'
import { useUpload } from './useUpload'

const RING_R = 11
const RING_LENGTH = 2 * Math.PI * RING_R

function Ring({ fraction, tone }: { fraction: number; tone: 'busy' | 'ok' | 'err' }) {
  return (
    <span className={tone === 'busy' ? 'ring-wrap' : `ring-wrap ${tone}`} aria-hidden="true">
      <svg className="ring" viewBox="0 0 28 28">
        <circle className="tr" cx="14" cy="14" r={RING_R} />
        <circle className="fg" cx="14" cy="14" r={RING_R} strokeDasharray={RING_LENGTH} strokeDashoffset={RING_LENGTH * (1 - fraction)} />
      </svg>
      {tone !== 'busy' && <Icon name={tone === 'ok' ? 'check' : 'alert'} size={14} />}
    </span>
  )
}

function UploadRow({ row }: { row: Row }) {
  const { t, tn, bytes, locale } = useI18n()
  const { cancel, retry } = useUpload()
  const online = useOnline()
  const percent = new Intl.NumberFormat(INTL_TAGS[locale], { style: 'percent' }).format(row.size > 0 ? row.loaded / row.size : 0)
  const shown = row.savedAs ?? row.name
  const running = row.status === 'uploading' || row.status === 'waiting'

  let status: string
  let tone = ''
  if (row.status === 'done') {
    status = row.kind === 'folder' ? tn('upload.files_done', row.count, { done: row.done }) : t('upload.status.done')
    tone = ' ok'
  } else if (row.status === 'failed') {
    status = t(row.error ?? 'error.UNKNOWN')
    tone = ' err'
  } else if (row.status === 'cancelled') status = t('upload.status.cancelled')
  else if (row.status === 'waiting') status = online ? t('upload.status.waiting') : t('upload.status.paused')
  else status = row.kind === 'folder' ? tn('upload.files_done', row.count, { done: row.done }) : `${percent} · ${t('upload.progress', { loaded: bytes(row.loaded), total: bytes(row.size) })}`

  return (
    <li className={row.status === 'cancelled' ? 'up-row off' : 'up-row'}>
      <Icon name={row.kind === 'folder' ? 'folder' : 'file'} size={22} />
      <div className="t">
        <div className="nm trunc">{shown}</div>
        {row.status === 'uploading' && (
          <div className="pbar" role="progressbar" aria-label={shown} aria-valuemin={0} aria-valuemax={100} aria-valuenow={row.size > 0 ? Math.round((row.loaded / row.size) * 100) : 0}>
            <i style={{ width: row.size > 0 ? `${(row.loaded / row.size) * 100}%` : '0%' }} />
          </div>
        )}
        <div className={`st${tone}`}>{status}</div>
        {wasRenamed(row.name, row.savedAs) && <div className="st">{t('upload.saved_as', { name: row.savedAs ?? '' })}</div>}
      </div>
      {running && (
        <button type="button" className="ibtn" aria-label={t('upload.cancel', { name: row.name })} onClick={() => cancel(row.transferIds)}>
          <Icon name="x" size={16} />
        </button>
      )}
      {(row.status === 'failed' || row.status === 'cancelled') && (
        <button type="button" className="btn text" aria-label={t('upload.retry', { name: row.name })} onClick={() => retry(row.transferIds)}>
          {t('retry')}
        </button>
      )}
    </li>
  )
}

/** Bottom-end card listing every upload of this session (design/app-spec.md section 4). */
export function UploadPanel() {
  const { t, tn, locale } = useI18n()
  const { state, cancel, clear } = useUpload()
  const [minimized, setMinimized] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [now, setNow] = useState(() => Date.now())
  const active = isActive(state.transfers)

  // The remaining-time estimate ticks once a second while something is running.
  useEffect(() => {
    if (!active) return
    const timer = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(timer)
  }, [active])

  const rows = toRows(state)
  if (rows.length === 0) return null

  const { loaded, total, remaining } = overall(state.transfers)
  const failed = state.transfers.some((item) => item.status === 'failed')
  const done = state.transfers.some((item) => item.status === 'done')

  let title: string
  if (active) title = tn('upload.title.active', remaining)
  else if (failed) title = t('upload.title.failed')
  else if (done) title = t('upload.title.done')
  else title = t('upload.title.cancelled')

  const left = active && state.startedAt !== null ? remainingSeconds(loaded, total, now - state.startedAt) : null
  const last = state.transfers[state.transfers.length - 1]
  const subtitle = [
    left === null ? null : t('upload.left', { time: new Intl.NumberFormat(INTL_TAGS[locale], { style: 'unit', unitDisplay: 'narrow', unit: durationParts(left).unit }).format(durationParts(left).value) }),
    t('upload.to', { folder: last.destName }),
  ]
    .filter((part): part is string => part !== null)
    .join(' · ')

  const close = () => {
    if (!active) clear()
    else if (remaining > 1) setConfirming(true)
    else {
      cancel(state.transfers.map((item) => item.id))
      clear()
    }
  }

  const cancelAll = () => {
    cancel(state.transfers.map((item) => item.id))
    clear()
    setConfirming(false)
  }

  const tone = active ? 'busy' : failed ? 'err' : 'ok'

  return (
    <section className="up" aria-label={t('upload.panel')}>
      <div className="up-h">
        <Ring fraction={total > 0 ? loaded / total : 0} tone={tone} />
        <div className="t">
          <b className="trunc" role="status">
            {title}
          </b>
          <span className="trunc">{subtitle}</span>
        </div>
        <button type="button" className="ibtn" aria-expanded={!minimized} aria-label={minimized ? t('upload.expand') : t('upload.minimize')} onClick={() => setMinimized(!minimized)}>
          <Icon name="chevD" size={18} className={minimized ? 'flip' : undefined} />
        </button>
        <button type="button" className="ibtn" aria-label={t('upload.close')} onClick={close}>
          <Icon name="x" size={18} />
        </button>
      </div>
      {!minimized && (
        <ul className="up-list">
          {rows.map((row) => (
            <UploadRow key={row.id} row={row} />
          ))}
        </ul>
      )}
      {confirming && (
        <Dialog title={t('upload.cancel_all.title')} onClose={() => setConfirming(false)}>
          <p>{t('upload.cancel_all.text', { n: remaining })}</p>
          <div className="row-end">
            <button type="button" className="btn plain" onClick={() => setConfirming(false)}>
              {t('upload.cancel_all.keep')}
            </button>
            <button type="button" className="btn primary" onClick={cancelAll}>
              {t('upload.cancel_all.confirm')}
            </button>
          </div>
        </Dialog>
      )}
    </section>
  )
}
