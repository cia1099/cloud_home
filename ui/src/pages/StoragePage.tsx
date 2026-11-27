import { getUsage, type Usage } from '../api/usage'
import { Icon } from '../components/Icon'
import { ListSkeleton, LoadError, StateView } from '../components/StateView'
import { useRemote } from '../data/useRemote'
import type { MessageKey } from '../i18n/messages/en'
import { useI18n } from '../i18n/useI18n'
import { isLow, percentOf, segments, type SegmentId } from '../storage/model'
import { useTween } from '../storage/useTween'
import { useStreamEvent } from '../sse/useEventStream'

const LABELS: Record<SegmentId, MessageKey> = {
  mine: 'storage.yours',
  trash: 'storage.trash',
  others: 'storage.others',
  free: 'storage.available',
}

function isUsage(value: unknown): value is Usage {
  return typeof value === 'object' && value !== null && 'used_bytes' in value && 'volume_total_bytes' in value
}

function Meter({ usage }: { usage: Usage }) {
  const { t, tn, bytes } = useI18n()
  const parts = segments(usage)
  const total = usage.volume_total_bytes
  const driveUsed = useTween(Math.max(0, total - usage.volume_available_bytes))
  return (
    <>
      <section className="scard" aria-labelledby="storage-used">
        <p id="storage-used" className="big-num num">{bytes(driveUsed)}</p>
        <p className="small">{t('storage.used', { used: bytes(total - usage.volume_available_bytes), total: bytes(total) })}</p>
        <div
          className="sbar"
          role="img"
          aria-label={`${t('storage.bar')}: ${parts.map((p) => `${t(LABELS[p.id])} ${bytes(p.bytes)}`).join(', ')}`}
        >
          {parts.map((p) => (
            <i key={p.id} className={`seg-${p.id}`} style={{ width: `${percentOf(p.bytes, total)}%` }} />
          ))}
        </div>
        <ul className="legend">
          {parts.map((p) => (
            <li key={p.id}>
              <i className={`dot-${p.id}`} aria-hidden="true" />
              <span className="lname">{t(LABELS[p.id])}</span>
              <span className="num small">{bytes(p.bytes)}</span>
            </li>
          ))}
        </ul>
        <p className="small">{t('storage.free', { free: bytes(usage.volume_available_bytes) })} · {tn('storage.files', usage.file_count)}</p>
      </section>
      {isLow(usage) && (
        <div className="notice" role="status">
          <Icon name="alert" size={18} />
          <span>
            <strong>{t('storage.low')}</strong> {t('storage.low.text')}
          </span>
        </div>
      )}
    </>
  )
}

/** Storage usage: drive totals and how much is yours, in the Trash, other accounts' or free. Updates live. */
export function StoragePage() {
  const { t } = useI18n()
  const { state, reload, replace } = useRemote(getUsage)
  useStreamEvent('usage', (data) => {
    if (isUsage(data)) replace(data)
  })

  let body
  if (state.status === 'loading') body = <ListSkeleton />
  else if (state.status === 'error') body = <LoadError error={state.error} onRetry={reload} />
  else if (!state.data.drive_available) {
    body = <StateView icon="drive" tone="warn" title={t('files.error_drive.title')} text={t('files.error_drive.text')} />
  } else {
    body = (
      <>
        <Meter usage={state.data} />
        <section className="cell-list">
          <div className="cell">
            <span className="lbl rowicon"><Icon name="drive" size={22} />{t('storage.drive')}</span>
            <span className="chip ok"><Icon name="check" size={13} />{t('storage.connected')}</span>
          </div>
        </section>
      </>
    )
  }

  return (
    <div className="page">
      <h1 className="h1">{t('storage.title')}</h1>
      {body}
    </div>
  )
}
