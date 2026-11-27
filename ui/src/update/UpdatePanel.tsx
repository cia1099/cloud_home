import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { ApiError } from '../api/client'
import { errorKey } from '../api/errors'
import {
  checkForUpdate,
  fetchUpdateStatus,
  installUpdate,
  saveUpdateSettings,
  type Channel,
  type Mode,
  type UpdateSettings,
  type UpdateStatus,
} from '../api/update'
import { Dialog } from '../components/Dialog'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useToast } from '../toast/useToast'
import { reloadForUpdate } from './reload'
import { isStale, safeNotesUrl } from './version'
import './update.css'

const POLL_MS = 2000
const WORKING: ReadonlySet<string> = new Set(['downloading', 'installing', 'restarting'])
const STEPS = ['downloading', 'installing', 'restarting'] as const

/** Two-hour windows, as in the design; a window set some other way stays selectable. */
const WINDOWS = Array.from({ length: 12 }, (_, i) => {
  const hh = (n: number) => String((n * 2) % 24).padStart(2, '0')
  return `${hh(i)}:00-${hh(i + 1)}:00`
})

const v = (version: string) => `v${version}`

function Hero({ title, sub, action, busy }: { title: string; sub: ReactNode; action?: ReactNode; busy?: boolean }) {
  return (
    <div className="upd-card upd-hero">
      <span className="upd-logo" aria-hidden="true">
        <Icon name="cloud" size={34} />
      </span>
      <div className="upd-hero-text">
        <h2>{title}</h2>
        <p className="hint">{sub}</p>
      </div>
      {busy ? <Icon name="refresh" size={24} className="spin upd-spin" /> : action}
    </div>
  )
}

export function UpdatePanel({ password, first }: { password: string; first: UpdateStatus }) {
  const { t, date } = useI18n()
  const toast = useToast()
  const [status, setStatus] = useState(first)
  const [action, setAction] = useState<'check' | 'install' | 'settings' | null>(null)
  const [confirmForce, setConfirmForce] = useState(false)
  // The install was accepted here (or one is running): follow it until the new version answers.
  const [following, setFollowing] = useState(WORKING.has(first.phase))
  // The box stopped answering while following: it is restarting.
  const [away, setAway] = useState(false)
  const [rolledBack, setRolledBack] = useState(false)
  const wasAway = useRef(false)

  const fail = useCallback((cause: unknown) => toast({ text: t(errorKey(cause)) }), [t, toast])

  useEffect(() => {
    if (!following) return
    let stopped = false
    const poll = async () => {
      try {
        const next = await fetchUpdateStatus(password)
        if (stopped) return
        if (isStale(__APP_VERSION__, next.current_version)) {
          reloadForUpdate(next.current_version)
          return
        }
        setStatus(next)
        setAway(false)
        if (!WORKING.has(next.phase)) {
          setFollowing(false)
          // Back on the old version with nothing failed to report: the new one did not start and the box went back.
          setRolledBack(wasAway.current && next.phase !== 'failed')
          wasAway.current = false
        }
      } catch (cause) {
        if (stopped) return
        if (cause instanceof ApiError && cause.status === 401) {
          // The password changed on the box: stop following; the user can lock and unlock again.
          setFollowing(false)
          return
        }
        wasAway.current = true
        setAway(true)
      }
    }
    const timer = window.setInterval(() => void poll(), POLL_MS)
    return () => {
      stopped = true
      window.clearInterval(timer)
    }
  }, [following, password])

  const check = () => {
    setAction('check')
    checkForUpdate(password)
      .then(setStatus, fail)
      .finally(() => setAction(null))
  }

  const install = (force: boolean) => {
    setAction('install')
    setRolledBack(false)
    installUpdate(password, force)
      .then((next) => {
        setStatus(next)
        setFollowing(true)
      })
      .catch((cause: unknown) => {
        if (cause instanceof ApiError && cause.code === 'UPDATE_TRANSFERS_ACTIVE') setConfirmForce(true)
        else fail(cause)
      })
      .finally(() => setAction(null))
  }

  const change = (patch: Partial<Pick<UpdateSettings, 'mode' | 'window' | 'channel'>>) => {
    setAction('settings')
    saveUpdateSettings(password, patch)
      .then((next) => {
        setStatus(next)
        toast({ text: t('admin.saved') })
      }, fail)
      .finally(() => setAction(null))
  }

  const { available, settings } = status
  const current = v(status.current_version)
  const working = WORKING.has(status.phase) || following
  const notes = available ? safeNotesUrl(available.notes_url) : null
  const installing = action === 'install'

  let hero: ReactNode
  if (!status.enabled) {
    hero = <Hero title={t('admin.disabled.title')} sub={t('admin.disabled.text')} />
  } else if (working) {
    const label = away ? t('admin.phase.waiting') : t(`admin.phase.${status.phase === 'idle' ? 'downloading' : status.phase}` as MessageKey)
    hero = <Hero title={available ? `${current} → ${v(available.version)}` : current} sub={label} busy />
  } else if (status.phase === 'failed' || rolledBack) {
    hero = (
      <div className="upd-card upd-hero">
        <span className="upd-logo warn" aria-hidden="true">
          <Icon name="alert" size={34} />
        </span>
        <div className="upd-hero-text">
          <h2>{t('admin.phase.failed')}</h2>
          <p className="hint">{t('admin.phase.failed_text', { err: status.last_error ?? '' })}</p>
        </div>
        <button type="button" className="btn primary" disabled={installing} onClick={() => (available ? install(false) : check())}>
          {t('retry')}
        </button>
      </div>
    )
  } else if (status.phase === 'checking' || action === 'check') {
    hero = <Hero title={t('admin.checking')} sub={`${t('admin.current_version')} ${current}`} busy />
  } else if (available) {
    hero = (
      <Hero
        title={`${t('admin.update_available')}: ${v(available.version)}`}
        sub={`${t('admin.current_version')} ${current}`}
        action={
          <button type="button" className="btn primary lg" disabled={installing} onClick={() => install(false)}>
            {t('admin.install')}
          </button>
        }
      />
    )
  } else {
    hero = (
      <Hero
        title={t('admin.up_to_date')}
        sub={`${t('admin.current_version')} ${current}${status.last_checked_at ? ` · ${t('admin.checked_at', { t: date(status.last_checked_at) })}` : ''}`}
        action={
          <button type="button" className="btn plain" onClick={check}>
            {t('admin.check_now')}
          </button>
        }
      />
    )
  }

  const step = away ? 2 : Math.max(0, STEPS.indexOf(status.phase as (typeof STEPS)[number]))
  const windows = WINDOWS.includes(settings.window) ? WINDOWS : [settings.window, ...WINDOWS]
  const locked = !status.enabled || action === 'settings'

  return (
    <>
      <section aria-labelledby="upd-h" className="upd-stack">
        <h2 id="upd-h" className="group-label">
          {t('admin.software_update')}
        </h2>
        {hero}
        {status.enabled && working && (
          <div className="upd-card upd-progress">
            <div className="upd-indet" role="progressbar" aria-label={t('admin.software_update')} />
            <ol className="upd-steps">
              {STEPS.map((name, i) => (
                <li key={name} className={i < step ? 'done' : i === step ? 'now' : ''} aria-current={i === step ? 'step' : undefined}>
                  <span className="n" aria-hidden="true">
                    {i < step ? <Icon name="check" size={14} /> : i + 1}
                  </span>
                  {t(`admin.phase.${name}`)}
                </li>
              ))}
            </ol>
            <p className="hint">{t('admin.update_covers')}</p>
          </div>
        )}
        {status.enabled && !working && available && status.phase !== 'failed' && (
          <div className="upd-card upd-notes">
            {notes && (
              <a href={notes} target="_blank" rel="noopener noreferrer">
                {t('admin.release_notes')}
                <Icon name="chevR" size={12} />
              </a>
            )}
            <p className="hint">{t('admin.update_covers')}</p>
          </div>
        )}
      </section>

      <section aria-labelledby="upd-s">
        <h2 id="upd-s" className="group-label">
          {t('admin.update_settings')}
        </h2>
        <div className="cell-list">
          <div className="cell">
            <span className="lbl">{t('admin.mode')}</span>
            <div className="seg" role="group" aria-label={t('admin.mode')}>
              {(['manual', 'auto'] as const satisfies readonly Mode[]).map((mode) => (
                <button key={mode} type="button" aria-pressed={settings.mode === mode} disabled={locked} onClick={() => settings.mode !== mode && change({ mode })}>
                  {t(mode === 'manual' ? 'admin.manual' : 'admin.automatic')}
                </button>
              ))}
            </div>
          </div>
          <div className="cell">
            <span className="upd-grow">
              <label className="lbl" htmlFor="upd-window">
                {t('admin.window')}
              </label>
              <span className="hint upd-block">{t('admin.window_hint')}</span>
            </span>
            <select id="upd-window" className="input upd-select" value={settings.window} disabled={locked} onChange={(e) => change({ window: e.target.value })}>
              {windows.map((w) => (
                <option key={w} value={w}>
                  {w.replace('-', ' – ')}
                </option>
              ))}
            </select>
          </div>
          <div className="cell">
            <span className="upd-grow">
              <span className="lbl upd-block">{t('admin.channel')}</span>
              <span className="hint upd-block">{t('admin.channel_hint')}</span>
            </span>
            <div className="seg" role="group" aria-label={t('admin.channel')}>
              {(['stable', 'beta'] as const satisfies readonly Channel[]).map((channel) => (
                <button key={channel} type="button" aria-pressed={settings.channel === channel} disabled={locked} onClick={() => settings.channel !== channel && change({ channel })}>
                  {t(channel === 'stable' ? 'admin.channel.stable' : 'admin.channel.beta')}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      {confirmForce && (
        <Dialog title={t('admin.install_anyway.title')} alert onClose={() => setConfirmForce(false)}>
          <p className="hint">{t('admin.busy_transfers')}</p>
          <div className="row-end">
            <button type="button" className="btn plain" onClick={() => setConfirmForce(false)}>
              {t('files.cancel')}
            </button>
            <button
              type="button"
              className="btn danger"
              onClick={() => {
                setConfirmForce(false)
                install(true)
              }}
            >
              {t('admin.install_anyway')}
            </button>
          </div>
        </Dialog>
      )}
    </>
  )
}
