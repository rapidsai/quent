// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  define: {
    'import.meta.env.TEST': 'true',
    'import.meta.env.VITE_API_BASE_URL': '"http://localhost:8000/api"',
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}', 'packages/@quent/*/src/**/*.{test,spec}.{ts,tsx}'],
    exclude: ['node_modules', 'dist'],
    reporters: ['default', 'junit'],
    outputFile: {
      junit: 'junit.xml',
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'cobertura'],
      exclude: ['node_modules/', 'src/test/', 'src/routeTree.gen.ts', '**/*.d.ts', '**/*.config.*'],
    },
  },
  resolve: {
    // Mirror vite.config.ts: the workspace packages each resolve their own
    // copy of these (pnpm peer-hash duplicates). Without dedupe, a jotai
    // <Provider> in a test does NOT scope atoms used inside @quent/hooks —
    // they silently fall back to jotai's global default store and state
    // leaks between tests.
    dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router'],
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
