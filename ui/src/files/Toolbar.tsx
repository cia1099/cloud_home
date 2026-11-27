import type { KeyboardEvent } from 'react'
import { Link } from 'react-router'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { Crumb } from './model'
import type { View } from './prefs'

const folderPath = (id: string | null) => (id ? `/files/${encodeURIComponent(id)}` : '/files')

/** Every ancestor is a link. On a phone only a back chevron and the current folder remain (CSS). */
export function Breadcrumbs({ trail, onNavigate }: { trail: Crumb[]; onNavigate: () => void }) {
  const { t } = useI18n()
  const all: { id: string | null; name: string }[] = [{ id: null, name: t('nav.files') }, ...trail]
  const current = all[all.length - 1]
  const parent = all[all.length - 2]
  return (
    <nav className="crumbs" aria-label={t('files.breadcrumb')}>
      {parent && (
        <Link className="crumb-back ibtn" to={folderPath(parent.id)} aria-label={t('files.back')} onClick={onNavigate}>
          <Icon name="chevL" size={20} />
        </Link>
      )}
      {all.slice(0, -1).map((crumb) => (
        <span key={crumb.id ?? 'root'} className="crumb-part">
          <Link to={folderPath(crumb.id)} onClick={onNavigate}>
            {crumb.name}
          </Link>
          <Icon name="chevR" size={14} />
        </span>
      ))}
      <span className="cur" aria-current="page">
        {current.name}
      </span>
    </nav>
  )
}

export function SearchField({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const { t } = useI18n()
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && value) {
      event.preventDefault()
      onChange('')
    }
  }
  return (
    <div className="search" role="search">
      <Icon name="search" size={16} />
      <input
        type="search"
        value={value}
        placeholder={t('files.search.placeholder')}
        aria-label={t('files.search.placeholder')}
        maxLength={100}
        autoComplete="off"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
      {value && (
        <button type="button" className="x" aria-label={t('files.search.clear')} onClick={() => onChange('')}>
          <Icon name="x" size={12} />
        </button>
      )}
    </div>
  )
}

export function ViewToggle({ view, onChange }: { view: View; onChange: (view: View) => void }) {
  const { t } = useI18n()
  return (
    <div className="seg" role="group" aria-label={t('files.view_as')}>
      {(['grid', 'list'] as const).map((mode) => (
        <button key={mode} type="button" aria-pressed={view === mode} aria-label={t(mode === 'grid' ? 'files.view.grid' : 'files.view.list')} onClick={() => onChange(mode)}>
          <Icon name={mode} size={18} />
        </button>
      ))}
    </div>
  )
}

/** Replaces the toolbar while something is selected (accent tint). */
export function SelectionBar({
  count,
  single,
  onDownload,
  onShare,
  onRename,
  onMove,
  onTrash,
  onClear,
}: {
  count: number
  single: boolean
  onDownload: () => void
  onShare: () => void
  onRename: () => void
  onMove: () => void
  onTrash: () => void
  onClear: () => void
}) {
  const { t, tn } = useI18n()
  return (
    <div className="selbar">
      <span className="count" role="status">
        {tn('files.selected', count)}
      </span>
      <div className="acts">
        <button type="button" className="abtn" onClick={onDownload}>
          <Icon name="download" size={18} />
          {t('download.action')}
        </button>
        {single && (
          <button type="button" className="abtn" onClick={onShare}>
            <Icon name="share" size={18} />
            {t('share.action')}
          </button>
        )}
        {single && (
          <button type="button" className="abtn" onClick={onRename}>
            <Icon name="edit" size={18} />
            {t('files.rename')}
          </button>
        )}
        <button type="button" className="abtn" onClick={onMove}>
          <Icon name="move" size={18} />
          {t('files.move')}
        </button>
        <button type="button" className="abtn dng" onClick={onTrash}>
          <Icon name="trash" size={18} />
          {t('files.delete')}
        </button>
      </div>
      <span className="grow" />
      <button type="button" className="abtn" onClick={onClear}>
        {t('files.deselect')}
      </button>
    </div>
  )
}
