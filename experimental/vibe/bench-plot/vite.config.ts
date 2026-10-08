// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { liveReports } from './scripts/live.mjs';

export default defineConfig({
  base: './',
  plugins: [tailwindcss(), svelte(), liveReports()],
  build: { outDir: 'dist', emptyOutDir: true },
});
