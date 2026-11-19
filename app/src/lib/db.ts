// IndexedDB: `outbox` (commands by uuid), `rows` (synced records by id), `meta` (token, hlc).
type Store = 'outbox' | 'rows' | 'meta'

export const req = <T>(r: IDBRequest<T>) =>
  new Promise<T>((ok, fail) => {
    r.onsuccess = () => ok(r.result)
    r.onerror = () => fail(r.error)
  })

let db: Promise<IDBDatabase> | undefined
const open = () =>
  (db ??= new Promise((ok, fail) => {
    const r = indexedDB.open('orbit', 1)
    r.onupgradeneeded = () => {
      r.result.createObjectStore('outbox', { keyPath: 'uuid' })
      r.result.createObjectStore('rows', { keyPath: 'id' })
      r.result.createObjectStore('meta')
    }
    r.onsuccess = () => ok(r.result)
    r.onerror = () => {
      db = undefined // let the next call retry
      fail(r.error)
    }
  }))

/** Runs `fn` in one readwrite transaction; resolves after commit, aborts if `fn` throws. */
export async function tx<T>(stores: Store[], fn: (s: (name: Store) => IDBObjectStore) => Promise<T>): Promise<T> {
  const t = (await open()).transaction(stores, 'readwrite')
  const done = new Promise((ok, fail) => {
    t.oncomplete = ok
    t.onabort = () => fail(t.error)
  })
  try {
    const result = await fn((name) => t.objectStore(name))
    await done
    return result
  } catch (e) {
    done.catch(() => {})
    try {
      t.abort()
    } catch {
      // Already aborted by IndexedDB; keep the original error.
    }
    throw e
  }
}
