import { defineConfig } from '@playwright/test';

const baseURL = process.env.GNOMCODE_WEB_BASE_URL || 'http://127.0.0.1:3000';

export default defineConfig({
  testDir: './web-e2e',
  workers: 1,
  outputDir: './web-test-results',
  timeout: 45_000,
  use: { baseURL, viewport: { width: 1440, height: 900 }, trace: 'retain-on-failure' },
  // With GNOMCODE_WEB_BASE_URL the suite runs against an already running (e.g. production) server.
  webServer: process.env.GNOMCODE_WEB_BASE_URL ? undefined : { command: 'npm run dev -- --hostname 127.0.0.1 --port 3000', url: baseURL, reuseExistingServer: !process.env.CI },
});
