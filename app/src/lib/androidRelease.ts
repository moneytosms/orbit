const LATEST_RELEASE = 'https://api.github.com/repos/moneytosms/orbit/releases/latest'
const STABLE_VERSION = /^(\d+)\.(\d+)\.(\d+)$/

export type AndroidShellRelease = { version: string; url: string }

/** Only offers a release with a real, correctly named APK asset. */
export function androidShellRelease(release: unknown, installedVersion: string): AndroidShellRelease | null {
  if (!release || typeof release !== 'object') return null
  const data = release as {
    tag_name?: unknown
    draft?: unknown
    prerelease?: unknown
    assets?: unknown
  }
  if (data.draft === true || data.prerelease === true || typeof data.tag_name !== 'string') return null
  const taggedVersion = /^v(\d+\.\d+\.\d+)$/.exec(data.tag_name)?.[1]
  const installed = STABLE_VERSION.exec(installedVersion)
  const tagged = taggedVersion && STABLE_VERSION.exec(taggedVersion)
  if (!tagged || !installed || compare(tagged, installed) <= 0 || !Array.isArray(data.assets)) return null

  const name = `Orbit-${taggedVersion}-android.apk`
  const asset = data.assets.find(
    (value) => value && typeof value === 'object' && (value as { name?: unknown }).name === name,
  ) as { browser_download_url?: unknown } | undefined
  if (typeof asset?.browser_download_url !== 'string') return null

  let url: URL
  try {
    url = new URL(asset.browser_download_url)
  } catch {
    return null
  }
  if (
    url.protocol !== 'https:' ||
    url.hostname !== 'github.com' ||
    url.pathname !== `/moneytosms/orbit/releases/download/${data.tag_name}/${name}` ||
    url.search ||
    url.hash
  ) {
    return null
  }
  return { version: taggedVersion, url: url.href }
}

export async function checkAndroidShellRelease(
  installedVersion: string,
  fetcher: typeof fetch = fetch,
): Promise<AndroidShellRelease | null> {
  const response = await fetcher(LATEST_RELEASE, {
    cache: 'no-store',
    headers: { Accept: 'application/vnd.github+json' },
  })
  if (!response.ok) return null
  return androidShellRelease(await response.json(), installedVersion)
}

function compare(left: RegExpExecArray, right: RegExpExecArray): number {
  for (let i = 1; i <= 3; i++) {
    const difference = Number(left[i]) - Number(right[i])
    if (difference) return difference
  }
  return 0
}
