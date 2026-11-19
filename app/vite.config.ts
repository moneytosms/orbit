import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'
import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'

const api = 'http://127.0.0.1:8080'
const proxy = { '/sync': api, '/auth': api, '/devices': api, '/export': api, '/disk': api, '/blobs': api }

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte(),
    VitePWA({
      registerType: 'autoUpdate',
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
          { src: 'icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        // The wasm core and the self-hosted font must be cached for the app to open offline.
        globPatterns: ['**/*.{js,css,html,wasm,woff2,svg,png}'],
      },
    }),
  ],
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
