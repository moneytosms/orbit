export const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const isAndroidTauri = () => inTauri() && /Android/i.test(navigator.userAgent)

export const isDesktopTauri = () => inTauri() && !isAndroidTauri()
