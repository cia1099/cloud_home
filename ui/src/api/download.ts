import { API_BASE, ApiError, isErrorEnvelope, reportUnauthorized } from './client'

export type ArchiveFormat = 'zip' | 'tar.gz'

/** The File System Access API is not in the TypeScript DOM lib yet; only the part used here. */
interface SaveFilePicker {
  showSaveFilePicker?: (options: { suggestedName: string }) => Promise<{ createWritable: () => Promise<WritableStream<Uint8Array>> }>
}

/** Single file: a real navigation download, so the browser streams it to disk. */
export function downloadFile(id: string): void {
  const link = document.createElement('a')
  link.href = `${API_BASE}/files/${encodeURIComponent(id)}/download`
  link.download = ''
  document.body.append(link)
  link.click()
  link.remove()
}

/** The archive endpoint takes a JSON body, which a plain form cannot send, so it is fetched. */
async function requestArchive(ids: readonly string[], format: ArchiveFormat): Promise<Response> {
  let response: Response
  try {
    response = await fetch(`${API_BASE}/files/download`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ids, format }),
    })
  } catch {
    throw new ApiError(0, 'NETWORK', 'network error')
  }
  if (response.ok) return response
  if (response.status === 401) reportUnauthorized()
  const payload: unknown = await response.json().catch(() => undefined)
  if (isErrorEnvelope(payload)) throw new ApiError(response.status, payload.error.code, payload.error.message)
  throw new ApiError(response.status, 'UNKNOWN', response.statusText)
}

/**
 * Downloads files and folders as one archive. Where the browser can write to a chosen file
 * (Chromium) the body is piped straight to disk; elsewhere it is collected into a Blob first.
 * Resolves once the server accepted the request; `finished` settles when the bytes are saved.
 */
async function saveArchive(ids: readonly string[], format: ArchiveFormat, name: string): Promise<{ finished: Promise<void> }> {
  const filename = `${name}.${format}`
  const { showSaveFilePicker } = window as unknown as SaveFilePicker
  if (!showSaveFilePicker) {
    const blob = await requestArchive(ids, format).then((r) => r.blob())
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    link.click()
    URL.revokeObjectURL(url)
    return { finished: Promise.resolve() }
  }
  // The picker needs the click's user activation, so it opens before the request.
  const handle = await showSaveFilePicker.call(window, { suggestedName: filename })
  const writable = await handle.createWritable()
  let response: Response
  try {
    response = await requestArchive(ids, format)
  } catch (cause) {
    await writable.abort().catch(() => undefined)
    throw cause
  }
  return { finished: (response.body as ReadableStream<Uint8Array>).pipeTo(writable) }
}

let running = 0
const watchers = new Set<() => void>()
const changed = () => watchers.forEach((notify) => notify())

/** Archive downloads in progress in this tab; a pending app update waits for them (R-027). */
export const downloadsRunning = (): number => running

export function subscribeDownloads(notify: () => void): () => void {
  watchers.add(notify)
  return () => {
    watchers.delete(notify)
  }
}

/** Counts the download from the moment it is requested until the last byte is saved (or it fails). */
export async function downloadArchive(ids: readonly string[], format: ArchiveFormat, name: string): Promise<{ finished: Promise<void> }> {
  running += 1
  changed()
  const end = () => {
    running -= 1
    changed()
  }
  try {
    const result = await saveArchive(ids, format, name)
    void result.finished.then(end, end)
    return result
  } catch (cause) {
    end()
    throw cause
  }
}
