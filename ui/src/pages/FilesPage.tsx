import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { createFolder, moveFile, renameFile, restoreFromTrash, trashFile, type FileItem } from '../api/files'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import { Icon, type IconName } from '../components/Icon'
import { Menu, type MenuAnchor, type MenuItem } from '../components/Menu'
import { Collection, type CollectionActions, type SelectMode } from '../files/Collection'
import { MoveDialog, NewFolderDialog } from '../files/Dialogs'
import { affectsFolder, rangeIds, sortItems, toggleId, type Sort, type SortKey } from '../files/model'
import { useSort, useView } from '../files/prefs'
import { Breadcrumbs, SearchField, SelectionBar, ViewToggle } from '../files/Toolbar'
import { useCoarse } from '../files/useCoarse'
import { useCrumbs } from '../files/useCrumbs'
import { useFolderListing, useSearchListing } from '../files/useListing'
import { INTL_TAGS } from '../i18n/locale'
import type { MessageKey } from '../i18n/messages/en'
import { useI18n } from '../i18n/useI18n'
import { useStreamEvent } from '../sse/useEventStream'
import { useToast } from '../toast/useToast'
import { useOnline } from '../shell/useOnline'
import { fromDrop } from '../upload/collect'
import { UploadButton } from '../upload/UploadButton'
import { useFileDrop } from '../upload/useFileDrop'
import { useUpload } from '../upload/useUpload'

const SEARCH_DEBOUNCE_MS = 250
const NO_SELECTION: ReadonlySet<string> = new Set()
const SORT_LABELS: Record<SortKey, MessageKey> = { name: 'files.col.name', modified: 'files.col.modified', size: 'files.col.size' }

interface MenuState {
  anchor: MenuAnchor
  items: MenuItem[]
  label: string
}

type DialogName = 'new-folder' | 'move' | null

function StateView({ icon, tone, title, text, children }: { icon: IconName; tone?: 'warn' | 'err' | 'gray'; title: string; text: string; children?: ReactNode }) {
  return (
    <div className="state">
      <span className={tone ? `big ${tone}` : 'big'} aria-hidden="true">
        <Icon name={icon} size={44} />
      </span>
      <h2>{title}</h2>
      <p>{text}</p>
      {children}
    </div>
  )
}

function Skeleton({ list }: { list: boolean }) {
  const { t } = useI18n()
  return (
    <div className={list ? 'skeleton skeleton-list' : 'skeleton skeleton-grid'} aria-busy="true" aria-label={t('loading')} role="status">
      {Array.from({ length: list ? 10 : 12 }, (_, i) => (
        <div key={i} className="sk" />
      ))}
    </div>
  )
}

/** The Files screen: browse a folder (or search results), select, rename, move, trash, create folders. */
export function FilesPage() {
  const { t, tn, locale } = useI18n()
  const toast = useToast()
  const navigate = useNavigate()
  const folderId = useParams().folderId ?? null
  const [params, setParams] = useSearchParams()
  const query = params.get('q')?.trim() ?? ''
  const searching = query !== ''
  const coarse = useCoarse()
  const online = useOnline()
  const { enqueue } = useUpload()

  const [view, setView] = useView()
  const [sort, setSort] = useSort()
  const [draft, setDraft] = useState(query)
  const [menu, setMenu] = useState<MenuState | null>(null)
  const [dialog, setDialog] = useState<DialogName>(null)
  const [renaming, setRenaming] = useState<{ id: string; error: string | null } | null>(null)

  const { listing: folderListing, reload: reloadFolder } = useFolderListing(folderId)
  const search = useSearchListing(searching ? query : null)
  const reloadSearch = search.reload
  const trail = useCrumbs(folderId)
  const listing = searching ? search.listing : folderListing

  // Selection belongs to one folder or query; changing either empties it without an effect.
  const scope = `${folderId ?? ''}|${query}`
  const [selected, setSelected] = useState<{ scope: string; ids: ReadonlySet<string> }>({ scope, ids: NO_SELECTION })
  const selection = selected.scope === scope ? selected.ids : NO_SELECTION
  const anchor = useRef<string | null>(null)
  const updateSelection = useCallback((ids: ReadonlySet<string>) => setSelected({ scope, ids }), [scope])

  const items = useMemo(() => (listing.status === 'ready' ? sortItems(listing.items, sort, INTL_TAGS[locale]) : []), [listing, sort, locale])
  const selectedItems = useMemo(() => items.filter((item) => selection.has(item.id)), [items, selection])

  const refresh = useCallback(() => {
    reloadFolder()
    reloadSearch()
  }, [reloadFolder, reloadSearch])

  useStreamEvent('files', (event) => {
    if (affectsFolder(event, folderId)) reloadFolder()
    if (searching) reloadSearch()
  })

  // The URL holds the committed query; the field holds what is typed until the debounce fires.
  useEffect(() => {
    const next = draft.trim()
    if (next === query) return
    const timer = window.setTimeout(() => {
      setParams(
        (prev) => {
          const copy = new URLSearchParams(prev)
          if (next) copy.set('q', next)
          else copy.delete('q')
          return copy
        },
        { replace: true },
      )
    }, next ? SEARCH_DEBOUNCE_MS : 0)
    return () => window.clearTimeout(timer)
  }, [draft, query, setParams])

  const closeMenu = useCallback(() => setMenu(null), [])

  const open = useCallback(
    (item: FileItem) => {
      if (item.file_type !== 'folder') return // preview and download arrive with R-009 / R-010
      setDraft('')
      void navigate(`/files/${encodeURIComponent(item.id)}`)
    },
    [navigate],
  )

  const select = useCallback(
    (item: FileItem, mode: SelectMode) => {
      if (mode === 'range') {
        updateSelection(new Set(rangeIds(items.map((i) => i.id), anchor.current ?? item.id, item.id)))
        return
      }
      anchor.current = item.id
      updateSelection(mode === 'toggle' ? toggleId(selection, item.id) : new Set([item.id]))
    },
    [items, selection, updateSelection],
  )

  const fail = useCallback((cause: unknown) => toast({ text: t(errorKey(cause)) }), [toast, t])

  const trash = useCallback(
    async (targets: readonly FileItem[]) => {
      const results = await Promise.allSettled(targets.map((item) => trashFile(item.id)))
      const done = targets.filter((_, i) => results[i].status === 'fulfilled')
      const failed = results.find((r) => r.status === 'rejected')
      updateSelection(NO_SELECTION)
      refresh()
      if (failed) fail(failed.reason)
      if (done.length === 0) return
      toast({
        text: tn('files.trashed', done.length),
        action: {
          label: t('files.undo'),
          run: () => {
            Promise.all(done.map((item) => restoreFromTrash(item.id)))
              .catch(fail)
              .finally(refresh)
          },
        },
      })
    },
    [refresh, fail, toast, tn, t, updateSelection],
  )

  const commitRename = useCallback(
    async (item: FileItem, name: string) => {
      const next = name.trim()
      if (!next || next === item.name) {
        setRenaming(null)
        return
      }
      try {
        await renameFile(item.id, next)
        setRenaming(null)
        refresh()
      } catch (cause) {
        setRenaming({ id: item.id, error: t(errorKey(cause)) })
      }
    },
    [refresh, t],
  )

  const targetsFor = (item: FileItem) => (selection.has(item.id) ? selectedItems : [item])

  const actions: CollectionActions = {
    select,
    selectAll: () => updateSelection(new Set(items.map((item) => item.id))),
    clearSelection: () => updateSelection(NO_SELECTION),
    open,
    startRename: (item) => setRenaming({ id: item.id, error: null }),
    commitRename: (item, name) => void commitRename(item, name),
    cancelRename: () => setRenaming(null),
    trashSelection: () => void trash(selectedItems),
    menu: (item, at) => {
      const targets = targetsFor(item)
      const single = targets.length === 1
      const entries: MenuItem[] = []
      if (single && item.file_type === 'folder') entries.push({ label: t('files.open'), icon: 'folder', onSelect: () => open(item) })
      if (single) entries.push({ label: t('files.rename'), icon: 'edit', onSelect: () => setRenaming({ id: item.id, error: null }) })
      entries.push({ label: t('files.move'), icon: 'move', onSelect: () => setDialog('move') })
      entries.push({ label: t('files.delete'), icon: 'trash', danger: true, separated: true, onSelect: () => void trash(targets) })
      setMenu({ anchor: at, items: entries, label: t('files.actions', { name: item.name }) })
    },
  }

  const destination = { id: folderId, name: trail.length > 0 ? trail[trail.length - 1].name : t('nav.files') }
  const { dragging, handlers: dropHandlers } = useFileDrop(online, (items) => {
    fromDrop(items).then((picked) => enqueue(picked, destination), fail)
  })

  const onSort = (key: SortKey) => setSort(sort.key === key ? { key, dir: sort.dir === 'asc' ? 'desc' : 'asc' } : { key, dir: 'asc' })

  const openSortMenu = (target: HTMLElement) => {
    const box = target.getBoundingClientRect()
    const pick = (next: Sort) => () => setSort(next)
    setMenu({
      anchor: { x: box.right - 220, y: box.bottom + 6 },
      label: t('files.sort_by'),
      items: [
        ...(['name', 'modified', 'size'] as const).map((key) => ({ label: t(SORT_LABELS[key]), checked: sort.key === key, onSelect: pick({ ...sort, key }) })),
        { label: t('files.sort.asc'), checked: sort.dir === 'asc', separated: true, onSelect: pick({ ...sort, dir: 'asc' }) },
        { label: t('files.sort.desc'), checked: sort.dir === 'desc', onSelect: pick({ ...sort, dir: 'desc' }) },
      ],
    })
  }

  const selecting = selectedItems.length > 0
  const barTrail = folderId && trail.length === 0 ? [{ id: folderId, name: '…' }] : trail

  let body: ReactNode
  if (listing.status === 'loading') {
    body = <Skeleton list={view === 'list'} />
  } else if (listing.status === 'error') {
    const code = listing.error instanceof ApiError ? listing.error.code : ''
    const retry = (
      <button type="button" className="btn primary" onClick={refresh}>
        {t('retry')}
      </button>
    )
    if (code === 'DRIVE_UNAVAILABLE') body = <StateView icon="drive" tone="warn" title={t('files.error_drive.title')} text={t('files.error_drive.text')}>{retry}</StateView>
    else if (code === 'NETWORK') body = <StateView icon="wifioff" tone="gray" title={t('files.offline.title')} text={t('files.offline.text')}>{retry}</StateView>
    else if (!searching && (code === 'NOT_FOUND' || code === 'BAD_REQUEST')) {
      body = (
        <StateView icon="folder" tone="gray" title={t('files.gone.title')} text={t(errorKey(listing.error))}>
          <button type="button" className="btn primary" onClick={() => void navigate('/files')}>
            {t('files.gone.action')}
          </button>
        </StateView>
      )
    } else body = <StateView icon="alert" tone="err" title={t('files.error.title')} text={t('files.error.text')}>{retry}</StateView>
  } else if (items.length === 0) {
    body = searching ? (
      <StateView icon="search" tone="gray" title={t('files.search.empty.title')} text={t('files.search.empty.text', { q: query })} />
    ) : (
      <StateView icon="folder" title={t('files.empty.title')} text={t('files.empty.text')}>
        <button type="button" className="btn primary" onClick={() => setDialog('new-folder')}>
          <Icon name="folderPlus" size={18} />
          {t('files.new_folder')}
        </button>
      </StateView>
    )
  } else {
    body = (
      <Collection
        items={items}
        view={view}
        sort={sort}
        onSort={onSort}
        selection={selection}
        renamingId={renaming?.id ?? null}
        renameError={renaming?.error ?? null}
        coarse={coarse}
        label={t('files.items')}
        actions={actions}
        footer={
          searching && search.hasMore ? (
            <div className="more-row">
              <button type="button" className="btn plain" disabled={search.loadingMore} onClick={search.loadMore}>
                {t('files.search.more')}
              </button>
            </div>
          ) : undefined
        }
      />
    )
  }

  return (
    <div className="fpage" {...dropHandlers}>
      <h1 className="sr-only">{t('files.title')}</h1>
      <div className={selecting ? 'bar sel' : 'bar'}>
        {selecting ? (
          <SelectionBar
            count={selectedItems.length}
            single={selectedItems.length === 1}
            onRename={() => setRenaming({ id: selectedItems[0].id, error: null })}
            onMove={() => setDialog('move')}
            onTrash={() => void trash(selectedItems)}
            onClear={() => updateSelection(NO_SELECTION)}
          />
        ) : (
          <>
            <Breadcrumbs trail={barTrail} onNavigate={() => setDraft('')} />
            <SearchField value={draft} onChange={setDraft} />
            <ViewToggle view={view} onChange={setView} />
            <button type="button" className="ibtn" aria-label={t('files.sort_by')} aria-haspopup="menu" onClick={(event) => openSortMenu(event.currentTarget)}>
              <Icon name="sort" size={20} />
            </button>
            <UploadButton disabled={!online} onPick={(picked) => enqueue(picked, destination)} onNewFolder={() => setDialog('new-folder')} />
          </>
        )}
      </div>

      {searching && listing.status === 'ready' && items.length > 0 && (
        <div className="results-head">
          <h2 className="h2">{t('files.search.results_for', { q: query })}</h2>
          <span className="small" role="status">
            {tn('files.found', search.total)}
          </span>
        </div>
      )}

      {body}

      {dragging && (
        <div className="drop" aria-hidden="true">
          <div className="in">
            <span className="dz">
              <Icon name="upload" size={40} />
            </span>
            <h2>{t('upload.drop.title')}</h2>
            <p>{t('upload.drop.text', { folder: destination.name })}</p>
          </div>
        </div>
      )}

      {menu && <Menu anchor={menu.anchor} items={menu.items} label={menu.label} onClose={closeMenu} />}
      {dialog === 'new-folder' && (
        <NewFolderDialog
          onClose={() => setDialog(null)}
          onCreate={async (name) => {
            const created = await createFolder(name, folderId)
            refresh()
            anchor.current = created.id
            updateSelection(new Set([created.id]))
          }}
        />
      )}
      {dialog === 'move' && (
        <MoveDialog
          moving={selectedItems}
          onClose={() => setDialog(null)}
          onMove={async (target) => {
            try {
              for (const item of selectedItems) await moveFile(item.id, target)
            } finally {
              refresh()
            }
            updateSelection(NO_SELECTION)
          }}
        />
      )}
    </div>
  )
}
