import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type ReactNode } from 'react'
import type { FileItem, SearchItem } from '../api/files'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { FileGlyph } from './FileGlyph'
import { chunk, gridColumns, visibleRange, type Sort, type SortKey } from './model'
import { RenameField } from './RenameField'
import type { View } from './prefs'

export type SelectMode = 'replace' | 'toggle' | 'range'

export interface CollectionActions {
  select: (item: FileItem, mode: SelectMode) => void
  selectAll: () => void
  clearSelection: () => void
  open: (item: FileItem) => void
  /** Opens the item menu at a viewport position. */
  menu: (item: FileItem, at: { x: number; y: number }) => void
  startRename: (item: FileItem) => void
  commitRename: (item: FileItem, name: string) => void
  cancelRename: () => void
  trashSelection: () => void
}

const LIST_ROW = 56
const GRID_ROW = 212 // 196 px tile + 16 px gap
const GRID_TOP = 20

/** The slot for the item's secondary line: where a search hit lives, otherwise nothing. */
const locationOf = (item: FileItem | SearchItem) => ('parent_path' in item ? item.parent_path : null)

interface Props {
  items: readonly (FileItem | SearchItem)[]
  view: View
  sort: Sort
  onSort: (key: SortKey) => void
  selection: ReadonlySet<string>
  renamingId: string | null
  renameError: string | null
  coarse: boolean
  label: string
  actions: CollectionActions
  /** Rendered after the last row (the "Show More" button of search results). */
  footer?: ReactNode
}

/** Windowed grid or list: only the rows near the viewport are in the DOM, so folders with thousands of items stay fast. */
export function Collection({ items, view, sort, onSort, selection, renamingId, renameError, coarse, label, actions, footer }: Props) {
  const { t, bytes, date } = useI18n()
  const scroller = useRef<HTMLDivElement>(null)
  const [metrics, setMetrics] = useState({ width: 0, height: 0, top: 0 })
  const [active, setActive] = useState(0)
  const focusAfterRender = useRef(false)

  useLayoutEffect(() => {
    const element = scroller.current
    if (!element) return
    const measure = () => setMetrics((m) => ({ ...m, width: element.clientWidth, height: element.clientHeight }))
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const columns = view === 'grid' ? gridColumns(metrics.width) : 1
  const rowHeight = view === 'grid' ? GRID_ROW : LIST_ROW
  const top = view === 'grid' ? GRID_TOP : 0
  const rows = view === 'grid' ? chunk(items, columns) : items.map((item) => [item])
  const header = view === 'list' ? 36 : 0
  const { first, last } = visibleRange(metrics.top - top - header, metrics.height, rowHeight, rows.length)
  const activeIndex = Math.min(active, Math.max(0, items.length - 1))

  // Keyboard moves scroll the target row into view, then focus it once it has been rendered.
  useEffect(() => {
    if (!focusAfterRender.current) return
    focusAfterRender.current = false
    scroller.current?.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`)?.focus()
  })

  const reveal = (index: number) => {
    const element = scroller.current
    if (!element) return
    // The sticky list header covers the first `header` px of the viewport.
    const rowTop = header + top + Math.floor(index / columns) * rowHeight
    if (rowTop < element.scrollTop + header) element.scrollTop = rowTop - header
    else if (rowTop + rowHeight > element.scrollTop + element.clientHeight) element.scrollTop = rowTop + rowHeight - element.clientHeight
    setMetrics((m) => ({ ...m, top: element.scrollTop }))
  }

  const moveTo = (index: number, event: KeyboardEvent) => {
    event.preventDefault()
    const clamped = Math.max(0, Math.min(items.length - 1, index))
    const item = items[clamped]
    if (!item) return
    setActive(clamped)
    reveal(clamped)
    focusAfterRender.current = true
    actions.select(item, event.shiftKey ? 'range' : 'replace')
  }

  const onKeyDown = (event: KeyboardEvent) => {
    const item = items[activeIndex]
    const mod = event.metaKey || event.ctrlKey
    const horizontal = view === 'grid'
    switch (event.key) {
      case 'ArrowDown':
        return moveTo(activeIndex + columns, event)
      case 'ArrowUp':
        return moveTo(activeIndex - columns, event)
      case 'ArrowRight':
        if (horizontal) moveTo(activeIndex + 1, event)
        return
      case 'ArrowLeft':
        if (horizontal) moveTo(activeIndex - 1, event)
        return
      case 'Home':
        return moveTo(0, event)
      case 'End':
        return moveTo(items.length - 1, event)
      case 'a':
        if (mod) {
          event.preventDefault()
          actions.selectAll()
        }
        return
      case 'Enter':
        if (item) {
          event.preventDefault()
          actions.open(item)
        }
        return
      case ' ':
        if (item) {
          event.preventDefault()
          actions.select(item, 'toggle')
        }
        return
      case 'F2':
        if (item) {
          event.preventDefault()
          actions.startRename(item)
        }
        return
      case 'Delete':
      case 'Backspace':
        if (selection.size > 0) {
          event.preventDefault()
          actions.trashSelection()
        }
        return
      case 'Escape':
        return actions.clearSelection()
    }
  }

  const renderItem = (item: FileItem | SearchItem, index: number) => {
    const selected = selection.has(item.id)
    const renaming = renamingId === item.id
    const folder = item.file_type === 'folder'
    const place = locationOf(item)
    const sub = place === null ? null : t('files.search.location', { path: place || t('nav.files') })
    const size = folder ? '' : bytes(item.size_bytes)
    const name = renaming ? (
      <RenameField item={item} error={renameError} onCommit={(value) => actions.commitRename(item, value)} onCancel={actions.cancelRename} />
    ) : (
      item.name
    )

    const onClick = (event: MouseEvent) => {
      setActive(index)
      if (coarse) {
        if (selection.size === 0) actions.open(item)
        else actions.select(item, 'toggle')
      } else if (event.shiftKey) actions.select(item, 'range')
      else if (event.metaKey || event.ctrlKey) actions.select(item, 'toggle')
      else actions.select(item, 'replace')
    }
    const common = {
      role: 'option',
      'aria-selected': selected,
      'data-index': index,
      tabIndex: index === activeIndex ? 0 : -1,
      onClick,
      onDoubleClick: () => !coarse && !renaming && actions.open(item),
      onContextMenu: (event: MouseEvent) => {
        event.preventDefault()
        if (!selected) actions.select(item, 'replace')
        actions.menu(item, { x: event.clientX, y: event.clientY })
      },
    }
    const tick = (
      <button
        type="button"
        className="tick"
        aria-label={t('files.select_item', { name: item.name })}
        tabIndex={-1}
        onClick={(event) => {
          event.stopPropagation()
          setActive(index)
          actions.select(item, 'toggle')
        }}
      >
        <Icon name="check" size={14} />
      </button>
    )
    const more = (
      <button
        type="button"
        className="ibtn more"
        aria-label={t('files.actions', { name: item.name })}
        aria-haspopup="menu"
        tabIndex={-1}
        onClick={(event) => {
          event.stopPropagation()
          const box = event.currentTarget.getBoundingClientRect()
          if (!selected) actions.select(item, 'replace')
          actions.menu(item, { x: box.left, y: box.bottom + 4 })
        }}
      >
        <Icon name="more" size={18} />
      </button>
    )

    if (view === 'grid') {
      return (
        <div key={item.id} className={selection.size > 0 ? 'tile selecting' : 'tile'} {...common}>
          {tick}
          {more}
          <div className={folder ? 'thumb folder' : 'thumb'}>
            <FileGlyph item={item} size={folder ? 96 : 56} thumbnail />
          </div>
          <div className="nm">{name}</div>
          <div className="mt">{sub ?? (folder ? date(item.updated_at) : size)}</div>
        </div>
      )
    }
    return (
      <div key={item.id} className="lrow" {...common}>
        <span className="cb" aria-hidden="true">
          <Icon name="check" size={14} />
        </span>
        <span className="nm">
          <span className={folder ? 'ico f' : 'ico'}>
            <FileGlyph item={item} size={32} thumbnail />
          </span>
          <span className="rn">
            <span className="rn-name">{name}</span>
            {sub && <span className="sub">{sub}</span>}
            <span className="sub compact">{[folder ? null : size, date(item.updated_at)].filter(Boolean).join(' · ')}</span>
          </span>
        </span>
        <span className="meta col-date">{date(item.updated_at)}</span>
        <span className="meta col-size num">{folder ? '—' : size}</span>
        {more}
      </div>
    )
  }

  const sortButton = (key: SortKey, text: string, className?: string) => (
    <span role="columnheader" className={className} aria-sort={sort.key === key ? (sort.dir === 'asc' ? 'ascending' : 'descending') : 'none'}>
      <button type="button" onClick={() => onSort(key)}>
        {text}
        {sort.key === key && <Icon name="chevD" size={14} className={sort.dir === 'asc' ? 'flip' : undefined} />}
      </button>
    </span>
  )

  return (
    <div ref={scroller} className="scroller" onScroll={(event) => setMetrics((m) => ({ ...m, top: event.currentTarget.scrollTop }))}>
      {view === 'list' && (
        <div className="lhead" role="row">
          <span />
          {sortButton('name', t('files.col.name'))}
          {sortButton('modified', t('files.col.modified'), 'col-date')}
          {sortButton('size', t('files.col.size'), 'col-size')}
          <span />
        </div>
      )}
      <div
        role="listbox"
        aria-multiselectable="true"
        aria-label={label}
        className={view === 'grid' ? 'virtual grid' : 'virtual'}
        style={{ height: top + rows.length * rowHeight }}
        onKeyDown={onKeyDown}
      >
        <div className={view === 'grid' ? 'window grid-rows' : 'window'} style={{ transform: `translateY(${top + first * rowHeight}px)` }}>
          {rows.slice(first, last).map((row, offset) => (
            <div key={first + offset} className="vrow" style={{ height: rowHeight, gridTemplateColumns: view === 'grid' ? `repeat(${columns}, minmax(0, 1fr))` : undefined }}>
              {row.map((item, col) => renderItem(item, (first + offset) * columns + col))}
            </div>
          ))}
        </div>
      </div>
      {footer}
    </div>
  )
}
