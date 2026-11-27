import { useId, useState, type FormEvent } from 'react'
import { createShare, listDirectory, type Share } from '../api/shares'
import { errorKey } from '../api/errors'
import type { FileItem } from '../api/files'
import { Dialog } from '../components/Dialog'
import { Icon } from '../components/Icon'
import { useRemote } from '../data/useRemote'
import type { MessageKey } from '../i18n/messages/en'
import { useI18n } from '../i18n/useI18n'
import { useToast } from '../toast/useToast'
import { EXPIRY_CHOICES, expiryToIso, shareUrl, type Expiry } from './model'

const EXPIRY_LABELS: Record<Expiry, MessageKey> = {
  '1d': 'share.expiry.1d',
  '7d': 'share.expiry.7d',
  '30d': 'share.expiry.30d',
  never: 'share.expiry.never',
}

type Audience = 'link' | 'people'

/** Share one file or folder: a public link, or specific people on this Cloud Home. Two phases: options, then result. */
export function ShareDialog({ item, onClose }: { item: Pick<FileItem, 'id' | 'name'>; onClose: () => void }) {
  const { t } = useI18n()
  const toast = useToast()
  const ids = useId()
  const directory = useRemote(listDirectory)
  const [audience, setAudience] = useState<Audience>('link')
  const [expiry, setExpiry] = useState<Expiry>('7d')
  const [download, setDownload] = useState(true)
  const [chosen, setChosen] = useState<ReadonlySet<string>>(new Set())
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<Share | null>(null)

  const people = directory.state.status === 'ready' ? directory.state.data : []
  const needsPeople = audience === 'people' && chosen.size === 0

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (needsPeople) return
    setBusy(true)
    setError(null)
    try {
      setResult(
        await createShare({
          file_id: item.id,
          can_download: download,
          expires_at: expiryToIso(expiry, Date.now()),
          recipient_ids: audience === 'people' ? [...chosen] : [],
        }),
      )
    } catch (cause) {
      setError(t(errorKey(cause)))
      setBusy(false)
    }
  }

  const toggle = (id: string) =>
    setChosen((prev) => {
      const next = new Set(prev)
      if (!next.delete(id)) next.add(id)
      return next
    })

  if (result) {
    const link = result.token ? shareUrl(window.location.origin, result.token) : null
    const copy = () => {
      if (link) void navigator.clipboard.writeText(link).then(() => toast({ text: t('share.link_copied') }), () => toast({ text: t('error.UNKNOWN') }))
    }
    return (
      <Dialog title={t('share.dialog.title', { name: item.name })} onClose={onClose}>
        {link ? (
          <>
            <div className="field">
              <label htmlFor={`${ids}-link`}>{t('share.link')}</label>
              <input id={`${ids}-link`} className="input" readOnly value={link} onFocus={(event) => event.currentTarget.select()} />
            </div>
            <p className="hint">{t('share.link.note')}</p>
          </>
        ) : (
          <p role="status">{t('share.shared_with_names', { names: result.recipients.map((p) => p.display_name).join(', ') })}</p>
        )}
        <div className="row-end">
          {link && (
            <button type="button" className="btn tint" onClick={copy}>
              <Icon name="copy" size={18} />
              {t('share.copy_link')}
            </button>
          )}
          <button type="button" className="btn primary" onClick={onClose}>
            {t('share.done')}
          </button>
        </div>
      </Dialog>
    )
  }

  return (
    <Dialog title={t('share.dialog.title', { name: item.name })} onClose={onClose}>
      <form className="dialog-form" onSubmit={(event) => void submit(event)}>
        <div className="field">
          <span id={`${ids}-aud`} className="label">{t('share.audience')}</span>
          <div className="seg" role="radiogroup" aria-labelledby={`${ids}-aud`}>
            {(['link', 'people'] as const).map((value) => (
              <button key={value} type="button" role="radio" aria-checked={audience === value} onClick={() => setAudience(value)}>
                {t(value === 'link' ? 'share.audience.link' : 'share.audience.people')}
              </button>
            ))}
          </div>
          {audience === 'people' && <p className="hint">{t('share.audience.people_hint')}</p>}
        </div>

        {audience === 'people' && (
          <div className="picker" role="group" aria-label={t('share.people.list')}>
            {directory.state.status === 'loading' && <p className="pad-s hint">{t('loading')}</p>}
            {directory.state.status === 'error' && <p className="pad-s hint err" role="alert">{t(errorKey(directory.state.error))}</p>}
            {directory.state.status === 'ready' && people.length === 0 && <p className="pad-s hint">{t('share.people.empty')}</p>}
            {people.map((person) => (
              <label key={person.id} className="picker-row">
                <input type="checkbox" checked={chosen.has(person.id)} onChange={() => toggle(person.id)} />
                <span className="picker-name">{person.display_name}</span>
              </label>
            ))}
          </div>
        )}

        <div className="field">
          <label htmlFor={`${ids}-exp`}>{t('share.expiry')}</label>
          <select id={`${ids}-exp`} className="input" value={expiry} onChange={(event) => setExpiry(event.target.value as Expiry)}>
            {EXPIRY_CHOICES.map((choice) => (
              <option key={choice} value={choice}>
                {t(EXPIRY_LABELS[choice])}
              </option>
            ))}
          </select>
        </div>

        <div className="switch-row">
          <div>
            <label htmlFor={`${ids}-dl`} className="lbl">{t('share.allow_download')}</label>
            <p className="hint">{t('share.allow_download.hint')}</p>
          </div>
          <input id={`${ids}-dl`} type="checkbox" role="switch" className="switch" checked={download} onChange={(event) => setDownload(event.target.checked)} />
        </div>

        {needsPeople && <p className="hint">{t('share.people.none_chosen')}</p>}
        {error && (
          <p className="hint err" role="alert">
            <Icon name="alert" size={14} />
            {error}
          </p>
        )}
        <div className="row-end">
          <button type="button" className="btn plain" onClick={onClose}>
            {t('files.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={busy || needsPeople}>
            {t(audience === 'link' ? 'share.create_link' : 'share.share')}
          </button>
        </div>
      </form>
    </Dialog>
  )
}
