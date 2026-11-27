import { useCallback } from 'react'
import { useParams } from 'react-router'
import { ApiError } from '../api/client'
import { getPublicShare, publicDownloadUrl } from '../api/shares'
import { AuthLayout } from '../auth/AuthLayout'
import { Icon } from '../components/Icon'
import { LoadError } from '../components/StateView'
import { useRemote } from '../data/useRemote'
import { FileGlyph } from '../files/FileGlyph'
import { useI18n } from '../i18n/useI18n'

/** `/s/:token`: the page anyone with the link sees. No session, no shell; it has its own language switch. */
export function PublicSharePage() {
  const { t, bytes } = useI18n()
  const token = useParams().token ?? ''
  const load = useCallback((signal: AbortSignal) => getPublicShare(token, signal), [token])
  const { state, reload } = useRemote(load)

  if (state.status === 'loading') {
    return (
      <AuthLayout>
        <div role="status" aria-label={t('loading')} className="centered-inline">
          <Icon name="spinner" size={32} className="spin" />
        </div>
      </AuthLayout>
    )
  }
  if (state.status === 'error') {
    const gone = state.error instanceof ApiError && state.error.code !== 'NETWORK' && state.error.status < 500
    return (
      <AuthLayout>
        {gone ? (
          <>
            <span className="statusmark err" aria-hidden="true"><Icon name="link" size={34} /></span>
            <h1>{t('public.gone.title')}</h1>
            <p className="sub">{t('public.gone.text')}</p>
          </>
        ) : (
          <LoadError error={state.error} onRetry={reload} />
        )}
      </AuthLayout>
    )
  }

  const share = state.data
  return (
    <AuthLayout>
      <div className="public-file">
        <FileGlyph item={{ id: '', name: share.name, file_type: share.file_type, mime_type: share.mime_type }} size={88} />
      </div>
      <h1>{share.name}</h1>
      <p className="sub">
        {t('public.shared_by', { name: share.shared_by })}
        {share.file_type === 'file' && ` · ${bytes(share.size_bytes)}`}
      </p>
      {share.file_type === 'file' &&
        (share.can_download ? (
          <a className="btn primary lg block" href={publicDownloadUrl(token)} download>
            <Icon name="download" size={20} />
            {t('public.download')}
          </a>
        ) : (
          <span className="chip public-chip">{t('shared.view_only')}</span>
        ))}
      <p className="auth-foot">{t('public.note')}</p>
    </AuthLayout>
  )
}
