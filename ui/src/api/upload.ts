import { API_BASE, ApiError, isErrorEnvelope, reportUnauthorized } from './client'
import type { FileItem } from './files'

/**
 * Uploads one file with `XMLHttpRequest` (fetch has no upload progress). The `File` goes into the
 * form as a reference, so the browser streams it from disk instead of loading it into memory.
 * Aborting through `signal` rejects with an `AbortError`.
 */
export function uploadFile(file: File, parentId: string | null, onProgress: (loaded: number) => void, signal: AbortSignal): Promise<FileItem> {
  return new Promise((resolve, reject) => {
    const aborted = () => new DOMException('aborted', 'AbortError')
    if (signal.aborted) {
      reject(aborted())
      return
    }
    const xhr = new XMLHttpRequest()
    const form = new FormData()
    if (parentId) form.append('parent_id', parentId)
    form.append('name', file.name)
    form.append('file', file, file.name)

    xhr.open('POST', `${API_BASE}/files/upload`)
    xhr.responseType = 'json'
    xhr.upload.onprogress = (event) => onProgress(event.loaded)
    xhr.onerror = () => reject(new ApiError(0, 'NETWORK', 'network error'))
    xhr.onabort = () => reject(aborted())
    xhr.onload = () => {
      const payload: unknown = xhr.response
      if (xhr.status >= 200 && xhr.status < 300 && typeof payload === 'object' && payload !== null && 'data' in payload) {
        resolve((payload as { data: FileItem }).data)
        return
      }
      if (xhr.status === 401) reportUnauthorized()
      // The body limit answers 413 with an empty body.
      if (xhr.status === 413) reject(new ApiError(413, 'PAYLOAD_TOO_LARGE', 'payload too large'))
      else if (isErrorEnvelope(payload)) reject(new ApiError(xhr.status, payload.error.code, payload.error.message))
      else reject(new ApiError(xhr.status, 'UNKNOWN', xhr.statusText))
    }
    signal.addEventListener('abort', () => xhr.abort(), { once: true })
    xhr.send(form)
  })
}
