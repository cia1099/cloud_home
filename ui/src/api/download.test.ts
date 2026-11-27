import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from './client'
import { downloadArchive } from './download'

function sink() {
  const chunks: Uint8Array[] = []
  const abort = vi.fn()
  const writable = new WritableStream<Uint8Array>({ write: (chunk) => void chunks.push(chunk), abort })
  const showSaveFilePicker = vi.fn(async () => ({ createWritable: async () => writable }))
  vi.stubGlobal('window', { showSaveFilePicker })
  return { chunks, abort, showSaveFilePicker }
}

afterEach(() => vi.unstubAllGlobals())

describe('downloadArchive', () => {
  it('posts ids and format, then streams the body into the chosen file', async () => {
    const out = sink()
    vi.stubGlobal('fetch', vi.fn(async () => new Response(new Uint8Array([1, 2, 3]))))
    const { finished } = await downloadArchive(['a', 'b'], 'tar.gz', 'Photos')
    await finished
    expect(out.showSaveFilePicker).toHaveBeenCalledWith({ suggestedName: 'Photos.tar.gz' })
    const [url, init] = vi.mocked(fetch).mock.calls[0]
    expect(url).toBe('/api/v1/files/download')
    expect(JSON.parse(init?.body as string)).toEqual({ ids: ['a', 'b'], format: 'tar.gz' })
    expect(out.chunks.flatMap((c) => [...c])).toEqual([1, 2, 3])
  })

  it('surfaces the API error code and discards the half-opened file', async () => {
    const out = sink()
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ error: { code: 'NOT_FOUND', message: 'x' } }), { status: 404 })))
    await expect(downloadArchive(['a'], 'zip', 'x')).rejects.toMatchObject({ code: 'NOT_FOUND' } satisfies Partial<ApiError>)
    expect(out.abort).toHaveBeenCalled()
  })
})
