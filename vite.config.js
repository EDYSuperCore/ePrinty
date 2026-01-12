import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

const host = process.env.TAURI_DEV_HOST
const port = process.env.TAURI_DEV_PORT

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: host || false,
    port: port ? parseInt(port) : 5173,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})

