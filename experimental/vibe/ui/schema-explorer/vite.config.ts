// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig(({ command }) => ({
  base: process.env.SCHEMA_EXPLORER_BASE ?? '/',
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
    dedupe: ['svelte'],
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
