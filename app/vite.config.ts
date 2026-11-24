import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { createHash } from 'node:crypto'
import { readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'

const api = 'http://127.0.0.1:8080'
const proxy = {
  '/version': api,
  '/sync': api,
  '/auth': api,
  '/devices': api,
  '/export': api,
  '/disk': api,
  '/blobs': api,
}

function appFingerprint() {
  const root = path.dirname(fileURLToPath(import.meta.url))
  const files = ['src', 'wasm', 'public', 'index.html', 'vite.config.ts', 'package.json', 'pnpm-lock.yaml']
    .flatMap((dir) => {
      if (!['src', 'wasm', 'public'].includes(dir)) return [dir]
      const walk = (base: string): string[] =>
        readdirSync(path.join(root, base), { withFileTypes: true }).flatMap((entry) => {
          const file = path.join(base, entry.name)
          return entry.isDirectory() ? walk(file) : [file]
        })
      return walk(dir)
    })
    .sort()
  const hash = createHash('sha256')
  for (const file of files) hash.update(file).update(readFileSync(path.join(root, file)))
  return hash.digest('hex').slice(0, 16)
}
const demo = !!process.env.VITE_DEMO
const uiHash = process.env.VITE_ORBIT_UI_HASH ?? appFingerprint()
const appVersion = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')).version

// https://vite.dev/config/
export default defineConfig({
  // The demo lives under the site (VITE_DEMO_BASE, e.g. /orbit/demo/) and registers no service worker.
  base: process.env.VITE_DEMO_BASE ?? '/',
  build: demo ? { outDir: 'dist-demo' } : {},
  plugins: [
    tailwindcss(),
    svelte(),
    ...(demo
      ? []
      : [
          VitePWA({
            registerType: 'prompt',
            manifest: {
              name: 'Orbit',
              short_name: 'Orbit',
              description: 'Tasks, goals and routines',
              theme_color: '#fbfaf8',
              background_color: '#fbfaf8',
              display: 'standalone',
              icons: [
                { src: 'icon-192.png', sizes: '192x192', type: 'image/png' },
                { src: 'icon-512.png', sizes: '512x512', type: 'image/png' },
                { src: 'icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
              ],
            },
            workbox: {
              importScripts: ['sw-notify.js'],
              // The wasm core and the self-hosted font must be cached for the app to open offline.
              globPatterns: ['**/*.{js,css,html,wasm,woff2,svg,png}'],
            },
          }),
        ]),
    {
      name: 'orbit-build-info',
      generateBundle() {
        this.emitFile({ type: 'asset', fileName: 'orbit-build.json', source: JSON.stringify({ ui: uiHash }) })
      },
    },
  ],
  define: { __ORBIT_APP_VERSION__: JSON.stringify(appVersion), __ORBIT_UI_HASH__: JSON.stringify(uiHash) },
  resolve: { alias: { $lib: path.resolve('./src/lib') } },
  // svelte-tiptap ships .svelte files, so Vite must prebundle its ProseMirror deps together or dev serves two copies.
  optimizeDeps: {
    exclude: ['svelte-tiptap'],
    include: [
      '@tiptap/core',
      '@tiptap/pm/state',
      '@tiptap/pm/view',
      '@tiptap/pm/model',
      '@tiptap/starter-kit',
      '@tiptap/markdown',
    ],
  },
  server: { proxy },
  preview: { proxy },
})
