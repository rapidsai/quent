// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repositoryRoot = fileURLToPath(new URL('../../../..', import.meta.url));

function quentCommit(): string | null {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], {
      cwd: repositoryRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return null;
  }
}

export default defineConfig(({ command }) => ({
  base: process.env.SCHEMA_EXPLORER_BASE ?? '/',
  define: {
    __QUENT_BUILD_INFO__: JSON.stringify({
      commit: quentCommit(),
      builtAt: new Date().toISOString(),
    }),
  },
  cacheDir: fileURLToPath(
    new URL('../node_modules/.vite/schema-explorer', import.meta.url),
  ),
  plugins: [tailwindcss(), svelte()],
  publicDir: false,
  server: {
    fs: {
      allow: [
        fileURLToPath(new URL('..', import.meta.url)),
        fileURLToPath(
          new URL('../../../../crates/schema/ts', import.meta.url),
        ),
        fileURLToPath(new URL('../../../../ui', import.meta.url)),
      ],
    },
  },
  resolve: {
    // CodeMirror extensions require the same state constructors across packages.
    dedupe: ['svelte', '@codemirror/state'],
    ...(command === 'serve'
      ? {
          alias: [
            {
              find: '@quent/schema-viewer/styles.css',
              replacement: fileURLToPath(
                new URL('../schema-viewer/src/styles.css', import.meta.url),
              ),
            },
            {
              find: '@quent/schema-viewer',
              replacement: fileURLToPath(
                new URL('../schema-viewer/src/index.ts', import.meta.url),
              ),
            },
          ],
        }
      : {}),
  },
  optimizeDeps:
    command === 'serve'
      ? {
          exclude: ['@quent/schema-viewer'],
        }
      : undefined,
}));
