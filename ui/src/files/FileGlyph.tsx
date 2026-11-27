import { useState } from 'react'
import { thumbnailUrl, type FileItem } from '../api/files'
import { Icon, type IconName } from '../components/Icon'
import { kindOf, type Kind } from './model'

const KIND_ICON: Record<Exclude<Kind, 'folder'>, IconName> = {
  image: 'image',
  video: 'video',
  audio: 'music',
  pdf: 'pdf',
  doc: 'doc',
  sheet: 'doc',
  zip: 'zip',
  file: 'file',
}

/** Folder icon, server thumbnail for images, or a coloured kind glyph. `size` is the glyph size in px. */
export function FileGlyph({ item, size, thumbnail = false }: { item: Pick<FileItem, 'id' | 'file_type' | 'name' | 'mime_type'>; size: number; thumbnail?: boolean }) {
  const [thumbFailed, setThumbFailed] = useState(false)
  const kind = kindOf(item)
  if (kind === 'folder') return <Icon name="folder" size={size} className="folder-glyph" />
  if (kind === 'image' && thumbnail && !thumbFailed) {
    return <img className="art" src={thumbnailUrl(item.id)} alt="" loading="lazy" draggable={false} onError={() => setThumbFailed(true)} />
  }
  return (
    <span className={`fileglyph kind-${kind}`} style={{ width: size, height: size }}>
      <Icon name={KIND_ICON[kind]} size={Math.round(size * 0.55)} />
    </span>
  )
}
