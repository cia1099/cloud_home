import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { downloadsRunning, subscribeDownloads } from '../api/download'
import { fetchServerVersion } from '../api/update'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { useOnline } from '../shell/useOnline'
import { backoffDelay } from '../sse/backoff'
import { useStreamStatus } from '../sse/useEventStream'
import { useToast } from '../toast/useToast'
import { isActive } from '../upload/model'
import { useUpload } from '../upload/useUpload'
import { clearReloadGuard, reloadAlreadyTried, reloadForUpdate, takeUpdatedVersion } from './reload'
import { isStale, reloadDecision } from './version'
import './update.css'

const IDLE_TICK_MS = 5000
/** A brief connection blip never flashes the overlay. */
const OVERLAY_GRACE_MS = 1000
/** After this long without the box coming back, the overlay offers Try Again. */
const GIVE_UP_MS = 3 * 60_000
const INPUTS = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'] as const

interface Overlay {
  /** When the box first stopped answering. */
  since: number
  /** When the next probe happens. */
  retryAt: number
}

/** The calm "updating, reconnecting" state: not an error, no stack trace, a visible countdown. */
function UpdatingOverlay({ overlay, onTryNow }: { overlay: Overlay; onTryNow: () => void }) {
  const { t } = useI18n()
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 500)
    return () => window.clearInterval(timer)
  }, [])
  const seconds = Math.max(1, Math.ceil((overlay.retryAt - now) / 1000))
  const gaveUp = now - overlay.since >= GIVE_UP_MS
  return (
    <div className="upd-overlay" role="status" aria-live="polite">
      <div className="upd-overlay-box">
        <span className="upd-logo big" aria-hidden="true">
          <Icon name="cloud" size={40} />
        </span>
        <h2>{t('update.updating')}</h2>
        <p>{t('update.updating_text')}</p>
        <div className="upd-indet" aria-hidden="true" />
        <span className="hint">{t('update.updating_retry', { s: seconds })}</span>
        {gaveUp && (
          <button type="button" className="btn primary" onClick={onTryNow}>
            {t('retry')}
          </button>
        )}
      </div>
    </div>
  )
}

/**
 * R-027: the bundle compares its own version with the box after every reconnect and on tab focus,
 * offers a reload (or reloads when the user is idle and nothing is transferring), treats a missing
 * hashed asset the same way, and shows a calm state while the box restarts.
 */
export function UpdateWatcher() {
  const { t } = useI18n()
  const toast = useToast()
  const stream = useStreamStatus()
  const online = useOnline()
  const { state: uploads } = useUpload()
  const downloads = useSyncExternalStore(subscribeDownloads, downloadsRunning)
  const busy = isActive(uploads.transfers) || downloads > 0

  const [serverVersion, setServerVersion] = useState<string | null>(null)
  const [assetMissing, setAssetMissing] = useState(false)
  const [dismissed, setDismissed] = useState<string | null>(null)
  const [idleMs, setIdleMs] = useState(0)
  const [overlay, setOverlay] = useState<Overlay | null>(null)
  const [probeRound, setProbeRound] = useState(0)
  const lastInput = useRef(Date.now())
  const downSince = useRef<number | null>(null)

  const stale = assetMissing || isStale(__APP_VERSION__, serverVersion)
  const key = serverVersion ?? '*'

  const checkVersion = useCallback(() => {
    void fetchServerVersion().then((version) => {
      if (version !== null) setServerVersion(version)
    })
  }, [])

  // "Updated to v…" once, in the tab that just reloaded into the new version.
  const announced = useRef(false)
  useEffect(() => {
    if (announced.current) return
    announced.current = true
    const version = takeUpdatedVersion()
    if (version) toast({ text: t('update.done', { v: `v${version}` }) })
  }, [t, toast])

  // After every (re)connect of the live stream, and whenever the tab comes back into view.
  useEffect(() => {
    if (stream === 'open') checkVersion()
  }, [stream, checkVersion])
  useEffect(() => {
    const onVisible = () => document.visibilityState === 'visible' && checkVersion()
    document.addEventListener('visibilitychange', onVisible)
    window.addEventListener('focus', checkVersion)
    return () => {
      document.removeEventListener('visibilitychange', onVisible)
      window.removeEventListener('focus', checkVersion)
    }
  }, [checkVersion])

  // A hashed asset that no longer exists (old tab, new box) takes the same path instead of a broken screen.
  useEffect(() => {
    const onPreloadError = (event: Event) => {
      event.preventDefault()
      setAssetMissing(true)
      checkVersion()
    }
    window.addEventListener('vite:preloadError', onPreloadError)
    return () => window.removeEventListener('vite:preloadError', onPreloadError)
  }, [checkVersion])

  useEffect(() => {
    if (!stale && serverVersion !== null) clearReloadGuard()
  }, [stale, serverVersion])

  // Idle time is only measured while an update is pending.
  useEffect(() => {
    const touch = () => {
      lastInput.current = Date.now()
    }
    for (const name of INPUTS) window.addEventListener(name, touch, { passive: true })
    return () => {
      for (const name of INPUTS) window.removeEventListener(name, touch)
    }
  }, [])
  useEffect(() => {
    if (!stale) return
    const measure = () => setIdleMs(Date.now() - lastInput.current)
    measure()
    const timer = window.setInterval(measure, IDLE_TICK_MS)
    return () => window.clearInterval(timer)
  }, [stale])

  const decision = reloadDecision({ stale, busy, idleMs, dismissed: dismissed === key, alreadyTried: reloadAlreadyTried(serverVersion) })
  useEffect(() => {
    if (decision === 'reload') reloadForUpdate(assetMissing ? null : serverVersion)
  }, [decision, serverVersion, assetMissing])

  // While the live stream is down and the browser is online the box is probably restarting: probe
  // /health with backoff (1 s, 2 s, 4 s ... 10 s) and show the calm overlay until it answers.
  useEffect(() => {
    if (stream === 'open' || !online) {
      downSince.current = null
      setOverlay(null)
      return
    }
    let attempt = 0
    let timer: number | undefined
    let cancelled = false
    const probe = async () => {
      const version = await fetchServerVersion()
      if (cancelled) return
      if (version !== null) {
        downSince.current = null
        setServerVersion(version)
        setOverlay(null)
        return
      }
      const delay = backoffDelay(attempt++)
      const now = Date.now()
      downSince.current ??= now
      setOverlay({ since: downSince.current, retryAt: now + delay })
      timer = window.setTimeout(() => void probe(), delay)
    }
    timer = window.setTimeout(() => void probe(), OVERLAY_GRACE_MS)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [stream, online, probeRound])

  return (
    <>
      {(decision === 'prompt' || decision === 'wait') && (
        <div className="upd-banner" role="status">
          <span className="upd-banner-icon" aria-hidden="true">
            <Icon name="upload" size={20} />
          </span>
          <div className="upd-banner-text">
            <b>{t('update.new_version')}</b>
            <span>
              {decision === 'wait'
                ? t('update.reload_after_transfer')
                : serverVersion && !assetMissing
                  ? t('update.new_version_text', { v: `v${serverVersion}` })
                  : t('update.new_version_text_generic')}
            </span>
          </div>
          {decision === 'prompt' && (
            <>
              <button type="button" className="btn primary" onClick={() => reloadForUpdate(assetMissing ? null : serverVersion)}>
                {t('update.reload')}
              </button>
              <button type="button" className="btn text" onClick={() => setDismissed(key)}>
                {t('update.later')}
              </button>
            </>
          )}
        </div>
      )}
      {overlay && <UpdatingOverlay overlay={overlay} onTryNow={() => setProbeRound((n) => n + 1)} />}
    </>
  )
}
