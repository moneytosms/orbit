import { describe, expect, it, vi } from 'vitest'
import { checkAndroidShellRelease } from './androidRelease'

describe('Android shell releases', () => {
  it('offers the exact tagged APK asset only when that release contains it', async () => {
    const fixture = {
      tag_name: 'v0.3.0',
      draft: false,
      prerelease: false,
      assets: [
        {
          name: 'Orbit-0.3.0-android.apk',
          browser_download_url: 'https://github.com/moneytosms/orbit/releases/download/v0.3.0/Orbit-0.3.0-android.apk',
        },
      ],
    }
    const fetcher = vi.fn(async () => new Response(JSON.stringify(fixture), { status: 200 }))

    await expect(checkAndroidShellRelease('0.2.0', fetcher)).resolves.toEqual({
      version: '0.3.0',
      url: 'https://github.com/moneytosms/orbit/releases/download/v0.3.0/Orbit-0.3.0-android.apk',
    })
    expect(fetcher).toHaveBeenCalledWith('https://api.github.com/repos/moneytosms/orbit/releases/latest', {
      cache: 'no-store',
      headers: { Accept: 'application/vnd.github+json' },
    })
  })

  it('does not claim a server-only release without the Android APK', async () => {
    const fixture = { tag_name: 'v0.4.0', assets: [{ name: 'Orbit_0.4.0_amd64.AppImage' }] }
    const fetcher = vi.fn(async () => new Response(JSON.stringify(fixture), { status: 200 }))

    await expect(checkAndroidShellRelease('0.2.0', fetcher)).resolves.toBeNull()
  })
})
