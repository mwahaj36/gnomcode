import { defineConfig } from '@playwright/test';

// Records README demo frames only; never part of the e2e gate.
export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.record.ts',
  workers: 1,
  use: { colorScheme: 'dark' },
});
