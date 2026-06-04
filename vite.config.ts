import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // 'test' is a vitest-only extension; cast to satisfy tsc when typechecking vite.config.ts
  // (vite's UserConfig doesn't declare it, but vitest merges it at runtime for `npm test`)
  test: {
    globals: true,
    environment: 'happy-dom',   // happier with modern ESM packages than jsdom in this stack
    setupFiles: ['./src/test/setup.ts'],
  },
} as any)
