// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  cacheDir: new URL(
    '../node_modules/.vite/schema-viewer-test',
    import.meta.url,
  ).pathname,
  resolve: {
    conditions: ['browser'],
    dedupe: ['svelte'],
  },
  plugins: [
    svelte({
      dynamicCompileOptions: ({ filename }) => ({
        customElement: isSchemaViewerElement(filename),
      }),
    }),
  ],
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/setup.ts'],
  },
});

function isSchemaViewerElement(filename: string): boolean {
  return [
    '/EntityGraph.svelte',
    '/EntityEvents.svelte',
    '/FsmDetails.svelte',
    '/RecordDetails.svelte',
    '/ResourceDetails.svelte',
  ].some((suffix) => filename.endsWith(suffix));
}
