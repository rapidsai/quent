// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { FullConfig } from '@playwright/test';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const e2eDir = path.dirname(fileURLToPath(import.meta.url));
const uiRoot = path.resolve(e2eDir, '..');
const dataDir = process.env.PLAYWRIGHT_E2E_DATA_DIR ?? path.join(uiRoot, '.e2e-data');

async function globalSetup(_config: FullConfig) {
  const script = spawn('bash', [path.join(e2eDir, 'start-e2e-server.sh')], {
    stdio: 'inherit',
    env: process.env,
  });

  const [code, signal] = (await once(script, 'exit')) as [number | null, NodeJS.Signals | null];
  if (code !== 0) {
    throw new Error(`start-e2e-server.sh failed with ${signal ?? `exit code ${code}`}`);
  }

  return async () => {
    const pid = parseInt(await readFile(path.join(dataDir, 'server.pid'), 'utf8'), 10);
    try {
      process.kill(-pid, 'SIGTERM');
    } catch {
      try {
        process.kill(pid, 'SIGTERM');
      } catch {
        // already gone
      }
    }
  };
}

export default globalSetup;
