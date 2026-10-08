// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { createServer } from 'vite';

const root = fileURLToPath(new URL('..', import.meta.url));

test('local server follows a new complete report and serves its download', { timeout: 10000 }, async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'quent-bench-live-'));
  const original = process.env.QUENT_BENCH_RESULTS;
  process.env.QUENT_BENCH_RESULTS = directory;
  const fixture = JSON.parse(await readFile(path.join(root, 'tests/fixtures/report.json'), 'utf8'));
  await writeFile(path.join(directory, 'old.json'), JSON.stringify(fixture));
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.ts'),
    server: { host: '127.0.0.1', port: 0 } });
  try {
    await server.listen();
    const address = server.httpServer.address();
    const base = `http://127.0.0.1:${address.port}`;
    const selection = () => fetch(`${base}/api/selection`).then((response) => response.json());
    assert.match((await selection()).current.url, /old\.json/);
    await writeFile(path.join(directory, 'new.json'), '{');
    await new Promise((resolve) => setTimeout(resolve, 150));
    assert.match((await selection()).current.url, /old\.json/);
    fixture.system.captured_at_unix_seconds += 1;
    await writeFile(path.join(directory, 'new.json'), JSON.stringify(fixture));
    let latest;
    for (let attempt = 0; attempt < 30; attempt++) {
      latest = await selection();
      if (latest.current.url.includes('new.json')) break;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    assert.match(latest.current.url, /new\.json/);
    assert.deepEqual(Object.keys(latest), ['current']);
    assert.equal((await fetch(`${base}${latest.current.url}`)).status, 200);
    assert.equal((await fetch(`${base}/api/report/old.json`)).status, 404);
  } finally {
    await server.close();
    if (original === undefined) delete process.env.QUENT_BENCH_RESULTS;
    else process.env.QUENT_BENCH_RESULTS = original;
    await rm(directory, { recursive: true, force: true });
  }
});
