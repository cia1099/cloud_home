import { useState } from 'react'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { useStreamEvent } from '../sse/useEventStream'

/** Shape of the `usage` SSE event (UsageResponse in the API). */
interface Usage {
  used_bytes: number
  volume_total_bytes: number
  drive_available: boolean
}

function isUsage(data: unknown): data is Usage {
  return typeof data === 'object' && data !== null && 'used_bytes' in data && 'drive_available' in data
}

/** Placeholder until the file browser (R-007) lands; shows live usage to prove the SSE connection. */
export function FilesPage() {
  const { t, bytes } = useI18n()
  const [usage, setUsage] = useState<Usage | null>(null)
  useStreamEvent('usage', (data) => {
    if (isUsage(data)) setUsage(data)
  })

  const percent = usage && usage.volume_total_bytes > 0 ? Math.min(100, (usage.used_bytes / usage.volume_total_bytes) * 100) : 0
  return (
    <div className="page">
      <h1 className="h1">{t('files.title')}</h1>
      <div className="state">
        <span className="big" aria-hidden="true">
          <Icon name="folder" size={44} />
        </span>
        <h2>{t('files.placeholder.title')}</h2>
        <p>{t('files.placeholder.text')}</p>
        {usage &&
          (usage.drive_available ? (
            <>
              <div
                className="progress"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(percent)}
                aria-label={t('files.usage', { used: bytes(usage.used_bytes), total: bytes(usage.volume_total_bytes) })}
              >
                <i style={{ width: `${percent}%` }} />
              </div>
              <p className="small num">{t('files.usage', { used: bytes(usage.used_bytes), total: bytes(usage.volume_total_bytes) })}</p>
            </>
          ) : (
            <span className="chip warn">{t('files.drive_missing')}</span>
          ))}
      </div>
    </div>
  )
}
