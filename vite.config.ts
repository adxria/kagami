import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
export default defineConfig(async ({ mode }) => ({
  plugins: [
    svelte(),
    ...(mode === 'test' ? [(await import('./tests/server.ts')).testServer()] : []),
  ],
  resolve: {
    alias: {
      $transport: fileURLToPath(
        new URL(
          mode === 'test' ? './tests/transport.ts' : './src/lib/transport.ts',
          import.meta.url,
        ),
      ),
    },
  },
  clearScreen: false,
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: { target: 'es2022' },
  test: { include: ['src/**/*.test.ts'] },
}));
