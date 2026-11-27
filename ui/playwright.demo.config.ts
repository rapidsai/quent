// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, devices } from '@playwright/test';

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:4173';
const recordVideo = process.env.PLAYWRIGHT_DEMO_RECORD === '1';

export default defineConfig({
  testDir: './e2e',
  testMatch: 'demo-gif.spec.ts',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  timeout: 120_000,
  reporter: [['list']],
  outputDir: './test-results/demo-gif',
  use: {
    ...devices['Desktop Chrome'],
    baseURL,
    colorScheme: 'dark',
    viewport: { width: 1490, height: 797 },
    video: recordVideo
      ? {
          mode: 'on',
          size: { width: 1490, height: 796 },
        }
      : 'off',
  },
  webServer: process.env.PLAYWRIGHT_BASE_URL
    ? undefined
    : {
        command: 'pnpm demo:build && pnpm preview --host 127.0.0.1 --port 4173 --strictPort',
        url: baseURL,
        reuseExistingServer: false,
        timeout: 600_000,
      },
  projects: [
    {
      name: 'chromium',
    },
  ],
});
