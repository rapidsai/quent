// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { watch } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readReports, selectLatestReport } from './reports.mjs';

const repositoryRoot = fileURLToPath(new URL('../../../..', import.meta.url));

export function liveReports() {
  return {
    name: 'quent-bench-live-reports',
    configureServer(server) {
      const directory = path.resolve(process.env.QUENT_BENCH_RESULTS ?? path.join(repositoryRoot, 'benchmarks', 'results'));
      let current = null;
      const cache = new Map();
      let pending = Promise.resolve();
      const scan = () => {
        pending = pending.then(async () => {
          const reports = await readReports(directory, (message) => server.config.logger.warn(message), cache);
          current = selectLatestReport(reports);
        }).catch((error) => server.config.logger.error(error.message));
      };
      scan();
      let watcher;
      try {
        watcher = watch(directory, scan);
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      const interval = setInterval(scan, 5000);
      server.httpServer?.on('close', () => {
        watcher?.close();
        clearInterval(interval);
      });
      server.middlewares.use('/api/selection', async (_req, res) => {
        await pending;
        res.setHeader('Content-Type', 'application/json');
        res.setHeader('Cache-Control', 'no-store');
        res.end(JSON.stringify({
          current: current && { url: `/api/report/${encodeURIComponent(current.name)}`, version: current.version },
        }));
      });
      server.middlewares.use('/api/report', async (req, res) => {
        await pending;
        let name;
        try {
          name = decodeURIComponent((req.url ?? '').slice(1).split('?')[0]);
        } catch {
          res.statusCode = 400;
          res.end('Invalid report name');
          return;
        }
        if (current?.name !== name) {
          res.statusCode = 404;
          res.end('Report not found');
          return;
        }
        const { createReadStream } = await import('node:fs');
        res.setHeader('Content-Type', 'application/json');
        res.setHeader('Content-Disposition', 'attachment; filename="quent-bench-report.json"');
        res.setHeader('Cache-Control', 'no-store');
        createReadStream(current.file).pipe(res);
      });
    },
  };
}
