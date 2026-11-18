import { fileURLToPath, URL } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import webfontDownload from 'vite-plugin-webfont-dl'

import catppuccin from './vite/catppuccin.ts'

export default defineConfig({
  // webfontDownload(): self-hosts the Google Fonts <link> found in index.html (zero config)
  plugins: [vue(), tailwindcss(), catppuccin(), webfontDownload()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: 5173,
  },
})
