// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, devices } from '@playwright/test';

const base = process.env.SCHEMA_EXPLORER_BASE ?? '/';
const baseURL = `http://127.0.0.1:4177${base}`;

export default defineConfig({
  testDir: './e2e',
  forbidOnly: !!process.env.CI,
  use: {
    baseURL,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'vite preview --host 127.0.0.1 --port 4177 --strictPort',
    url: baseURL,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
