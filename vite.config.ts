import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'happy-dom',   // happier with modern ESM packages than jsdom in this stack
    setupFiles: ['./src/test/setup.ts'],
  },
})
