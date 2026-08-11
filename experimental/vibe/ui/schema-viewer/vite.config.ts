// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  cacheDir: new URL(
    '../node_modules/.vite/schema-viewer',
    import.meta.url,
  ).pathname,
  resolve: {
    dedupe: ['svelte'],
  },
  plugins: [
    svelte({
      dynamicCompileOptions: ({ filename }) => ({
        customElement: isSchemaViewerElement(filename),
      }),
    }),
  ],
  build: {
    lib: {
      entry: {
        index: 'src/index.ts',
      },
      formats: ['es'],
      fileName: (_format, entryName) => `${entryName}.js`,
      cssFileName: 'schema-viewer',
    },
    rolldownOptions: {
      external: (id) => id === 'svelte' || id.startsWith('svelte/'),
    },
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
