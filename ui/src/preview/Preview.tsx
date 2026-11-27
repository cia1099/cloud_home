import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode, type TouchEvent } from 'react'
import { downloadUrl, rawUrl, thumbnailUrl, type FileItem } from '../api/files'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { neighbor, stripWindow, swipeDelta, type MediaKind } from './model'

const STRIP_RADIUS = 12

interface Props {
  item: FileItem
  kind: MediaKind
  /** Images in gallery order (just the item itself for audio and video). */
  gallery: readonly FileItem[]
  onGo: (id: string) => void
  onClose: () => void
  onShare: () => void
}

/** Full-stage dark preview of an image gallery, an audio file or a video (design/app-spec.md section 5). */
export function Preview({ item, kind, gallery, onGo, onClose, onShare }: Props) {
  const { t, bytes } = useI18n()
  const dialog = useRef<HTMLDialogElement>(null)
  const touchStart = useRef<{ x: number; y: number } | null>(null)
  const ids = gallery.map((g) => g.id)
  const index = ids.indexOf(item.id)
  const prevId = neighbor(ids, item.id, -1)
  const nextId = neighbor(ids, item.id, 1)

  useEffect(() => {
    const element = dialog.current
    if (!element) return
    element.showModal()
    return () => element.close()
  }, [])

  // Warm the cache for the neighbours so stepping is instant.
  useEffect(() => {
    for (const id of [prevId, nextId]) if (id && kind === 'image') new Image().src = rawUrl(id)
  }, [prevId, nextId, kind])

  const go = (id: string | null) => id && onGo(id)

  const onKeyDown = (event: KeyboardEvent) => {
    if (kind !== 'image' || event.altKey || event.ctrlKey || event.metaKey) return
    if (event.key === 'ArrowLeft') go(prevId)
    else if (event.key === 'ArrowRight') go(nextId)
  }

  const onTouchEnd = (event: TouchEvent) => {
    const start = touchStart.current
    touchStart.current = null
    const end = event.changedTouches[0]
    if (!start || !end) return
    const delta = swipeDelta(end.clientX - start.x, end.clientY - start.y)
    if (delta) go(neighbor(ids, item.id, delta))
  }

  const span = stripWindow(gallery.length, index, STRIP_RADIUS)
  const subtitle = kind === 'image' && gallery.length > 1 ? t('preview.n_of_m', { n: index + 1, m: gallery.length }) : bytes(item.size_bytes)

  return (
    <dialog
      ref={dialog}
      className="pv"
      aria-label={t('preview.label', { name: item.name })}
      onCancel={(event) => {
        event.preventDefault()
        onClose()
      }}
      onKeyDown={onKeyDown}
    >
      <div className="pv-top">
        <button type="button" className="ibtn" aria-label={t('preview.close')} onClick={onClose}>
          <Icon name="x" size={20} />
        </button>
        <div className="t">
          <span className="trunc">{item.name}</span>
          <small className="num" aria-live="polite">
            {subtitle}
          </small>
        </div>
        <button type="button" className="ibtn" aria-label={t('share.action')} onClick={onShare}>
          <Icon name="share" size={20} />
        </button>
        <a className="ibtn" href={downloadUrl(item.id)} download={item.name} aria-label={t('preview.download')}>
          <Icon name="download" size={20} />
        </a>
      </div>

      <MediaStage
        key={item.id}
        item={item}
        kind={kind}
        onTouchStart={(event) => {
          const touch = event.touches[0]
          touchStart.current = event.touches.length === 1 && touch ? { x: touch.clientX, y: touch.clientY } : null
        }}
        onTouchEnd={onTouchEnd}
      >
        {prevId && (
          <button type="button" className="ibtn pv-nav p" aria-label={t('preview.prev')} onClick={() => go(prevId)}>
            <Icon name="chevL" size={24} />
          </button>
        )}
        {nextId && (
          <button type="button" className="ibtn pv-nav n" aria-label={t('preview.next')} onClick={() => go(nextId)}>
            <Icon name="chevR" size={24} />
          </button>
        )}
      </MediaStage>

      {gallery.length > 1 && (
        <div className="strip" role="list" aria-label={t('preview.strip')}>
          {gallery.slice(span.from, span.to).map((g) => (
            <button
              key={g.id}
              type="button"
              role="listitem"
              aria-current={g.id === item.id}
              aria-label={g.name}
              ref={(button) => {
                if (g.id === item.id) button?.scrollIntoView({ block: 'nearest', inline: 'center' })
              }}
              onClick={() => go(g.id)}
            >
              <img className="art" src={thumbnailUrl(g.id)} alt="" draggable={false} />
            </button>
          ))}
        </div>
      )}
    </dialog>
  )
}

function MediaStage({ item, kind, children, onTouchStart, onTouchEnd }: { item: FileItem; kind: MediaKind; children: ReactNode; onTouchStart: (event: TouchEvent) => void; onTouchEnd: (event: TouchEvent) => void }) {
  const { t } = useI18n()
  const [failed, setFailed] = useState(false)
  const [loaded, setLoaded] = useState(false)
  const src = rawUrl(item.id)
  const fail = () => setFailed(true)

  let media
  if (failed) {
    media = (
      <div className="state pv-state">
        <span className="big gray" aria-hidden="true">
          <Icon name={kind === 'audio' ? 'music' : kind === 'video' ? 'video' : 'image'} size={44} />
        </span>
        <h2>{t('preview.error.title')}</h2>
        <p>{t('preview.error.text')}</p>
        <a className="btn primary" href={downloadUrl(item.id)} download={item.name}>
          <Icon name="download" size={16} />
          {t('preview.download')}
        </a>
      </div>
    )
  } else if (kind === 'image') {
    media = (
      <div className={loaded ? 'pv-img' : 'pv-img loading'}>
        <img src={src} alt={item.name} draggable={false} onLoad={() => setLoaded(true)} onError={fail} />
      </div>
    )
  } else if (kind === 'audio') {
    media = (
      <div className="audio-wrap">
        <span className="cover" aria-hidden="true">
          <Icon name="music" size={96} />
        </span>
        <h2>{item.name}</h2>
        <audio className="pv-audio" src={src} controls preload="metadata" autoFocus onError={fail} />
      </div>
    )
  } else {
    media = <video className="pv-video" src={src} controls preload="metadata" playsInline autoFocus onError={fail} />
  }

  return (
    <div className="pv-stage" onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
      {media}
      {children}
    </div>
  )
}
