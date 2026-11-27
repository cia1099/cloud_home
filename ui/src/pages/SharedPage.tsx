import { useCallback, useState } from 'react'
import { useSearchParams } from 'react-router'
import { errorKey } from '../api/errors'
import { getFile, type FileItem } from '../api/files'
import { listReceived, listShares, markSeen, receivedDownloadUrl, revokeShare, type ReceivedShare, type Share } from '../api/shares'
import { ConfirmDialog } from '../components/ConfirmDialog'
import { Icon } from '../components/Icon'
import { ListSkeleton, LoadError, StateView } from '../components/StateView'
import { VirtualRows } from '../components/VirtualRows'
import { useNow } from '../data/useNow'
import { useRemote } from '../data/useRemote'
import { FileGlyph } from '../files/FileGlyph'
import { useI18n } from '../i18n/useI18n'
import { expiresSoon, isExpired, shareUrl } from '../shares/model'
import { useStreamEvent } from '../sse/useEventStream'
import { useToast } from '../toast/useToast'

const ROW_HEIGHT = 76

interface OwnShare {
  share: Share
  file: FileItem
}

/** `GET /shares` has no file names, so each share's file is fetched; shares whose file is gone are left out. */
async function loadOwn(signal: AbortSignal): Promise<OwnShare[]> {
  const shares = await listShares(signal)
  const files = await Promise.all(shares.map((share) => getFile(share.file_id, signal).catch((cause: unknown) => {
    if (cause instanceof DOMException) throw cause
    return null
  })))
  return shares.flatMap((share, i) => {
    const file = files[i]
    return file ? [{ share, file }] : []
  })
}

function ExpiryChip({ expiresAt }: { expiresAt: string | null }) {
  const { t, date } = useI18n()
  const now = useNow()
  if (expiresAt === null) return <span className="chip">{t('shared.never_expires')}</span>
  if (isExpired(expiresAt, now)) return <span className="chip err"><Icon name="clock" size={13} />{t('shared.expired')}</span>
  return <span className={expiresSoon(expiresAt, now) ? 'chip warn' : 'chip'}>{t('shared.expires_on', { date: date(expiresAt) })}</span>
}

function OwnRow({ entry, onStop }: { entry: OwnShare; onStop: (entry: OwnShare) => void }) {
  const { t, tn } = useI18n()
  const toast = useToast()
  const { share, file } = entry
  const expired = isExpired(share.expires_at, useNow())
  const names = share.recipients.map((p) => p.display_name).join(', ')
  const copy = () => {
    if (share.token === null) return
    void navigator.clipboard.writeText(shareUrl(window.location.origin, share.token)).then(
      () => toast({ text: t('share.link_copied') }),
      () => toast({ text: t('error.UNKNOWN') }),
    )
  }
  return (
    <>
      <span className="rico"><FileGlyph item={file} size={40} /></span>
      <span className="rtext">
        <span className="rname">{file.name}</span>
        <span className="rmeta">
          <ExpiryChip expiresAt={share.expires_at} />
          <span className="chip">{share.can_download ? t('shared.downloads_on') : t('shared.view_only')}</span>
          {share.token !== null && <span className="small">{tn('shared.opened', share.access_count)}</span>}
          {names !== '' && <span className="small rclip">{t('shared.with_names', { names })}</span>}
        </span>
      </span>
      <span className="racts">
        {share.token !== null && (
          <button type="button" className="btn plain" disabled={expired} aria-label={`${t('share.copy_link')}: ${file.name}`} onClick={copy}>
            <Icon name="link" size={18} />
            <span className="btn-label">{t('share.copy_link')}</span>
          </button>
        )}
        <button type="button" className="btn plain danger-text" aria-label={`${t('shared.stop')}: ${file.name}`} onClick={() => onStop(entry)}>
          <Icon name="x" size={18} />
          <span className="btn-label">{t('shared.stop')}</span>
        </button>
      </span>
    </>
  )
}

function ReceivedRow({ item, onOpened }: { item: ReceivedShare; onOpened: () => void }) {
  const { t, tn, bytes, date } = useI18n()
  const unseen = item.seen_at === null
  const now = useNow()
  const opened = () => {
    if (unseen) void markSeen(item.id).catch(() => undefined).finally(onOpened)
  }
  const size = item.file_type === 'folder' ? tn('shared.folder_items', item.child_count ?? 0) : bytes(item.size_bytes)
  return (
    <>
      <span className="rico"><FileGlyph item={{ id: item.file_id, name: item.name, file_type: item.file_type, mime_type: item.mime_type }} size={40} /></span>
      <span className="rtext">
        <span className="rname">
          {unseen && (
            <>
              <span className="dot" aria-hidden="true" />
              <span className="sr-only">{t('shared.new')}</span>
            </>
          )}
          {item.name}
        </span>
        <span className="rmeta">
          <span className="small rclip">{[t('shared.from', { name: item.owner.display_name }), t('shared.on', { date: date(item.created_at) }), size].join(' · ')}</span>
          {!item.can_download && <span className="chip">{t('shared.view_only')}</span>}
          {expiresSoon(item.expires_at, now) && item.expires_at && <ExpiryChip expiresAt={item.expires_at} />}
        </span>
      </span>
      <span className="racts">
        {item.file_type === 'file' && item.can_download && (
          <a className="btn plain" href={receivedDownloadUrl(item.id, item.file_id)} download aria-label={`${t('shared.download')}: ${item.name}`} onClick={opened}>
            <Icon name="download" size={18} />
            <span className="btn-label">{t('shared.download')}</span>
          </a>
        )}
      </span>
    </>
  )
}

/** Shared by You (links and people you shared with) and Shared with You (`?tab=with`). */
export function SharedPage() {
  const { t } = useI18n()
  const toast = useToast()
  const [params, setParams] = useSearchParams()
  const tab = params.get('tab') === 'with' ? 'with' : 'by'
  const own = useRemote(loadOwn)
  const received = useRemote(listReceived)
  const [stopping, setStopping] = useState<OwnShare | null>(null)

  const reloadOwn = own.reload
  const reloadReceived = received.reload
  useStreamEvent('files', reloadOwn)
  useStreamEvent('share.received', reloadReceived)
  useStreamEvent('share.revoked', reloadReceived)

  const unseen = received.state.status === 'ready' && received.state.data.some((item) => item.seen_at === null)
  const choose = useCallback((next: 'by' | 'with') => setParams(next === 'with' ? { tab: 'with' } : {}, { replace: true }), [setParams])

  const active = tab === 'with' ? received : own
  let body
  if (active.state.status === 'loading') body = <ListSkeleton />
  else if (active.state.status === 'error') body = <LoadError error={active.state.error} onRetry={active.reload} />
  else if (tab === 'by') {
    const rows = (own.state.status === 'ready' ? own.state.data : [])
    body = rows.length === 0 ? (
      <StateView icon="share" tone="gray" title={t('shared.by.empty.title')} text={t('shared.by.empty.text')} />
    ) : (
      <VirtualRows items={rows} rowHeight={ROW_HEIGHT} itemKey={(entry) => entry.share.id} label={t('shared.tab.by')} render={(entry) => <OwnRow entry={entry} onStop={setStopping} />} />
    )
  } else {
    const rows = received.state.status === 'ready' ? received.state.data : []
    body = rows.length === 0 ? (
      <StateView icon="users" tone="gray" title={t('shared.with.empty.title')} text={t('shared.with.empty.text')} />
    ) : (
      <VirtualRows items={rows} rowHeight={ROW_HEIGHT} itemKey={(item) => item.id} label={t('shared.tab.with')} render={(item) => <ReceivedRow item={item} onOpened={reloadReceived} />} />
    )
  }

  return (
    <div className="lpage">
      <div className="phead">
        <h1 className="h1">{t('shared.title')}</h1>
        <div className="seg" role="tablist" aria-label={t('shared.tabs')}>
          <button type="button" role="tab" id="tab-by" aria-selected={tab === 'by'} aria-controls="shared-panel" onClick={() => choose('by')}>
            {t('shared.tab.by')}
          </button>
          <button type="button" role="tab" id="tab-with" aria-selected={tab === 'with'} aria-controls="shared-panel" onClick={() => choose('with')}>
            {t('shared.tab.with')}
            {unseen && (
              <>
                <span className="dot" aria-hidden="true" />
                <span className="sr-only">{t('shared.new')}</span>
              </>
            )}
          </button>
        </div>
      </div>
      <div id="shared-panel" role="tabpanel" aria-labelledby={`tab-${tab}`} className="lbody">
        {body}
      </div>
      {stopping && (
        <ConfirmDialog
          title={t('shared.stop.title', { name: stopping.file.name })}
          text={t(stopping.share.token === null ? 'shared.stop.text_people' : 'shared.stop.text')}
          confirmLabel={t('shared.stop')}
          onClose={() => setStopping(null)}
          onConfirm={() =>
            revokeShare(stopping.share.id)
              .then(() => toast({ text: t('shared.stopped') }), (cause: unknown) => toast({ text: t(errorKey(cause)) }))
              .finally(reloadOwn)
          }
        />
      )}
    </div>
  )
}
