// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, devices } from '@playwright/test';

const basePath = process.env.VITE_BASE_PATH || '/';
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? new URL(basePath, 'http://127.0.0.1:4173').href;

export default defineConfig({
  testDir: './e2e-simulator',
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [['list']],
  use: { baseURL, trace: 'on-first-retry' },
  webServer: process.env.PLAYWRIGHT_BASE_URL
    ? undefined
    : {
        command:
          'pnpm demo:build && pnpm exec vite preview --config vite.simulator.config.ts --host 127.0.0.1 --port 4173 --strictPort',
        url: baseURL,
        reuseExistingServer: !process.env.CI,
        timeout: 180_000,
      },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
