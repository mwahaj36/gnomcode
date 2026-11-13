import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.spec.ts',
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  timeout: 45_000,
  expect: { timeout: 10_000 },
  reporter: process.env.CI ? [['list'], ['github']] : 'list',
  use: {
    ...(process.env.GNOMCODE_E2E_CHANNEL ? { channel: process.env.GNOMCODE_E2E_CHANNEL } : {}),
    trace: 'retain-on-failure',
    viewport: { width: 1440, height: 900 },
  },
});
