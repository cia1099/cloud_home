import type { ReactNode } from 'react'
import { Icon, type IconName } from './Icon'
import { useI18n } from '../i18n/useI18n'
import { errorKey } from '../api/errors'
import { ApiError } from '../api/client'

/** Centered empty or error state: big tinted icon, title, one sentence and an optional action. */
export function StateView({ icon, tone, title, text, children }: { icon: IconName; tone?: 'warn' | 'err' | 'gray'; title: string; text: string; children?: ReactNode }) {
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

/** Placeholder rows with the footprint of a real list, announced as loading. */
export function ListSkeleton() {
  const { t } = useI18n()
  return (
    <div className="skeleton skeleton-list" aria-busy="true" aria-label={t('loading')} role="status">
      {Array.from({ length: 6 }, (_, i) => (
        <div key={i} className="sk" />
      ))}
    </div>
  )
}

/** The load-failure state of a page: offline, drive missing or generic, always with Try Again. */
export function LoadError({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const { t } = useI18n()
  const code = error instanceof ApiError ? error.code : ''
  const retry = (
    <button type="button" className="btn primary" onClick={onRetry}>
      {t('retry')}
    </button>
  )
  if (code === 'DRIVE_UNAVAILABLE') return <StateView icon="drive" tone="warn" title={t('files.error_drive.title')} text={t('files.error_drive.text')}>{retry}</StateView>
  if (code === 'NETWORK') return <StateView icon="wifioff" tone="gray" title={t('files.offline.title')} text={t('files.offline.text')}>{retry}</StateView>
  return <StateView icon="alert" tone="err" title={t('list.error.title')} text={t(errorKey(error))}>{retry}</StateView>
}
