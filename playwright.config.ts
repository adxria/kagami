import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  testMatch: '*.spec.ts',
  workers: 2,
  use: {
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 1280, height: 860 },
    timezoneId: 'UTC',
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'npm run dev -- --mode test',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: false,
  },
});
