// Attachments: content-addressed files kept in IndexedDB (`blobs`), uploaded before the commands that cite them.
import { req, tx } from './db'

export const MAX_BLOB_BYTES = 25 * 1024 * 1024
/** The local cache keeps uploaded files up to this much, evicting the least recently used. */
export const CACHE_BYTES = 500 * 1024 * 1024

type Entry = { hash: string; data: ArrayBuffer; type: string; size: number; used: number; uploaded: boolean }

const hex = (buf: ArrayBuffer) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('')

const mb = (n: number) => `${Math.round(n / 1024 / 1024)} MB`

/** Stores `file` locally and returns the markdown that cites it. Throws a readable error when too big. */
export async function addFile(file: File): Promise<{ hash: string; markdown: string }> {
  if (file.size > MAX_BLOB_BYTES)
    throw new Error(`"${file.name}" is ${mb(file.size)}; attachments can be at most ${mb(MAX_BLOB_BYTES)}.`)
  const data = await file.arrayBuffer()
  const hash = hex(await crypto.subtle.digest('SHA-256', data))
  await tx(['blobs'], async (s) => {
    const have = await req<Entry | undefined>(s('blobs').get(hash))
    if (have) s('blobs').put({ ...have, used: Date.now() })
    else
      s('blobs').put({
        hash,
        data,
        type: file.type,
        size: data.byteLength,
        used: Date.now(),
        uploaded: false,
      } satisfies Entry)
  })
  const name = (file.name || 'file').replace(/[[\]\\]/g, '')
  return { hash, markdown: `${file.type.startsWith('image/') ? '!' : ''}[${name}](blob:${hash})` }
}

/** PUTs every blob the server hasn't got yet. A 413 drops that blob and is reported through `reject`. */
export async function uploadPending(reject: (code: string) => void = () => {}) {
  const all = await tx(['blobs'], (s) => req<Entry[]>(s('blobs').getAll()))
  for (const e of all.filter((b) => !b.uploaded)) {
    const res = await fetch(`/blobs/${e.hash}`, {
      method: 'PUT',
      credentials: 'include',
      headers: { 'content-type': 'application/octet-stream' },
      body: e.data,
    })
    if (res.status === 401) return // flush reports the sign-out
    if (res.status === 413) {
      await tx(['blobs'], async (s) => void s('blobs').delete(e.hash))
      reject('file_too_large')
      continue
    }
    if (!res.ok) throw new Error(`blob upload: HTTP ${res.status}`)
    await tx(['blobs'], async (s) => {
      const cur = await req<Entry | undefined>(s('blobs').get(e.hash))
      if (cur) s('blobs').put({ ...cur, uploaded: true })
    })
  }
  await evict()
}

/** Drops the least recently used uploaded blobs until the cache fits. Pending ones are never dropped. */
async function evict() {
  await tx(['blobs'], async (s) => {
    const all = await req<Entry[]>(s('blobs').getAll())
    let total = all.reduce((n, b) => n + b.size, 0)
    for (const b of all.filter((b) => b.uploaded).sort((a, b) => a.used - b.used)) {
      if (total <= CACHE_BYTES) break
      s('blobs').delete(b.hash)
      total -= b.size
    }
  })
}

/** An object URL for the file, from the cache or else fetched and cached; undefined if unavailable. */
export async function blobUrl(hash: string): Promise<string | undefined> {
  const hit = await tx(['blobs'], async (s) => {
    const e = await req<Entry | undefined>(s('blobs').get(hash))
    if (e) s('blobs').put({ ...e, used: Date.now() })
    return e
  })
  if (hit) return URL.createObjectURL(new Blob([hit.data], { type: hit.type }))
  const res = await fetch(`/blobs/${hash}`, { credentials: 'include' }).catch(() => undefined)
  if (!res?.ok) return undefined
  const data = await res.arrayBuffer()
  const type = res.headers.get('content-type') ?? ''
  await tx(['blobs'], async (s) => {
    s('blobs').put({ hash, data, type, size: data.byteLength, used: Date.now(), uploaded: true } satisfies Entry)
  })
  await evict()
  return URL.createObjectURL(new Blob([data], { type }))
}
