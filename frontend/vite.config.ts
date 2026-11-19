import { fileURLToPath, URL } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import vuetify from 'vite-plugin-vuetify'
import webfontDownload from 'vite-plugin-webfont-dl'

import catppuccin from './vite/catppuccin.ts'

export default defineConfig({
  plugins: [
    vue(),
    // Auto-imports only the Vuetify components/directives actually used (with their CSS).
    vuetify({ autoImport: true }),
    tailwindcss(),
    catppuccin(),
    // Self-hosts the Google Fonts <link> found in index.html (zero config). Vuetify uses the
    // same font through `--v-font-body` in src/styles/main.css.
    webfontDownload(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
})
