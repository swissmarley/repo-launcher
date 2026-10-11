/// <reference types="vitest/config" />
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// Tauri expects a fixed dev port and must see Rust errors in the terminal.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/app/**', '**/target/**'] },
  },
  test: {
    environment: 'jsdom',
    // e2e/ holds the Playwright flows (`pnpm e2e`).
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
