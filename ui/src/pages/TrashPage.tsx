import { useCallback, useState } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { getFile, trashFile } from '../api/files'
import { deleteForever, emptyTrash, listTrash, restoreTrashItem, type TrashItem } from '../api/trash'
import { ConfirmDialog } from '../components/ConfirmDialog'
import { Icon } from '../components/Icon'
import { ListSkeleton, LoadError, StateView } from '../components/StateView'
import { VirtualRows } from '../components/VirtualRows'
import { useNow } from '../data/useNow'
import { useRemote } from '../data/useRemote'
import { FileGlyph } from '../files/FileGlyph'
import { useI18n } from '../i18n/useI18n'
import { useStreamEvent } from '../sse/useEventStream'
import { useToast } from '../toast/useToast'
import { daysLeft, isUrgent } from '../trash/model'

const ROW_HEIGHT = 68

type Confirm = { kind: 'one'; item: TrashItem } | { kind: 'all' } | null

function DaysChip({ expiresAt }: { expiresAt: string }) {
  const { t, tn } = useI18n()
  const days = daysLeft(expiresAt, useNow())
  const urgent = isUrgent(days)
  return (
    <span className={urgent ? 'chip warn' : 'chip'}>
      {urgent && <Icon name="clock" size={13} />}
      {days === 0 ? t('trash.today') : tn('trash.days_left', days)}
    </span>
  )
}

/** The Trash: restore, delete forever and empty; entries show how many days are left before they go. */
export function TrashPage() {
  const { t, tn, bytes, date } = useI18n()
  const toast = useToast()
  const { state, reload } = useRemote(listTrash)
  const [confirm, setConfirm] = useState<Confirm>(null)

  useStreamEvent('files', reload)

  const fail = useCallback((cause: unknown) => toast({ text: t(errorKey(cause)) }), [toast, t])

  const restore = useCallback(
    async (item: TrashItem) => {
      try {
        const file = await restoreTrashItem(item.id)
        let folder = t('nav.files')
        if (file.parent_id) folder = await getFile(file.parent_id).then((parent) => parent.name, () => folder)
        reload()
        toast({
          text: t('trash.restored_to', { folder }),
          action: {
            label: t('files.undo'),
            run: () => {
              trashFile(file.id).catch(fail).finally(reload)
            },
          },
        })
      } catch (cause) {
        // 404: the entry was already gone (for example purged on another device); the list is stale.
        if (!(cause instanceof ApiError && cause.status === 404)) fail(cause)
        reload()
      }
    },
    [reload, toast, fail, t],
  )

  const items = state.status === 'ready' ? state.data : []
  const totalBytes = items.reduce((sum, item) => sum + item.size_bytes, 0)

  let body
  if (state.status === 'loading') body = <ListSkeleton />
  else if (state.status === 'error') body = <LoadError error={state.error} onRetry={reload} />
  else if (items.length === 0) body = <StateView icon="trash" tone="gray" title={t('trash.empty.title')} text={t('trash.empty.text')} />
  else {
    body = (
      <VirtualRows
        items={items}
        rowHeight={ROW_HEIGHT}
        itemKey={(item) => item.id}
        label={t('trash.items')}
        render={(item) => (
          <>
            <span className="rico">
              <FileGlyph item={{ id: item.file_id, name: item.name, file_type: item.file_type, mime_type: null }} size={40} />
            </span>
            <span className="rtext">
              <span className="rname">{item.name}</span>
              <span className="rmeta">
                <span className="small">{[item.file_type === 'file' ? bytes(item.size_bytes) : null, t('trash.deleted_on', { date: date(item.deleted_at) })].filter(Boolean).join(' · ')}</span>
                <DaysChip expiresAt={item.expires_at} />
              </span>
            </span>
            <span className="racts">
              <button type="button" className="btn plain" aria-label={`${t('trash.restore')}: ${item.name}`} onClick={() => void restore(item)}>
                <Icon name="restore" size={18} />
                <span className="btn-label">{t('trash.restore')}</span>
              </button>
              <button type="button" className="btn plain danger-text" aria-label={`${t('trash.delete_forever')}: ${item.name}`} onClick={() => setConfirm({ kind: 'one', item })}>
                <Icon name="trash" size={18} />
                <span className="btn-label">{t('trash.delete_forever')}</span>
              </button>
            </span>
          </>
        )}
      />
    )
  }

  return (
    <div className="lpage">
      <div className="phead">
        <div>
          <h1 className="h1">{t('trash.title')}</h1>
          <p className="small">{t('trash.note')}</p>
        </div>
        {items.length > 0 && (
          <button type="button" className="btn plain danger-text" onClick={() => setConfirm({ kind: 'all' })}>
            {t('trash.empty_trash')}
          </button>
        )}
      </div>
      {body}
      {confirm?.kind === 'one' && (
        <ConfirmDialog
          title={t('trash.confirm_delete.title', { name: confirm.item.name })}
          text={t('trash.confirm_delete.text')}
          confirmLabel={t('trash.delete_forever')}
          onClose={() => setConfirm(null)}
          onConfirm={() => deleteForever(confirm.item.id).then(() => undefined, fail).finally(reload)}
        />
      )}
      {confirm?.kind === 'all' && (
        <ConfirmDialog
          title={t('trash.confirm_empty.title')}
          text={tn('trash.confirm_empty.text', items.length, { size: bytes(totalBytes) })}
          confirmLabel={t('trash.empty_trash')}
          onClose={() => setConfirm(null)}
          onConfirm={() => emptyTrash().then(() => undefined, fail).finally(reload)}
        />
      )}
    </div>
  )
}
