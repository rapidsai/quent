// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import path from 'path';
import { defineConfig, mergeConfig } from 'vite';
import uiConfig from './vite.config';

export default defineConfig(
  mergeConfig(uiConfig, {
    base: process.env.VITE_BASE_PATH || '/',
    root: path.resolve(__dirname, 'simulator'),
    publicDir: path.resolve(__dirname, 'public'),
    build: {
      outDir: path.resolve(__dirname, 'dist'),
      emptyOutDir: true,
    },
  })
);
