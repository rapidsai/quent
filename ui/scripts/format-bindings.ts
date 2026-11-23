// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { glob, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolveConfigFromFile } from 'vite-plus';
import { format } from 'vite-plus/fmt';

const configPath = fileURLToPath(new URL('../vite.config.ts', import.meta.url));
const { fmt: fmtConfig } = await resolveConfigFromFile(configPath);

if (!fmtConfig) {
  throw new Error('vite.config.ts must define fmt options');
}

const { ignorePatterns: _, ...formatOptions } = fmtConfig;
let fileCount = 0;

for await (const fileName of glob('generated/ts-bindings/**/*.ts')) {
  const sourceText = await readFile(fileName, 'utf8');
  const result = await format(fileName, sourceText, formatOptions);

  if (result.errors.length > 0) {
    throw new Error(
      `Failed to format ${fileName}: ${result.errors.map(error => error.message).join('; ')}`
    );
  }

  if (result.code !== sourceText) {
    await writeFile(fileName, result.code);
  }
  fileCount += 1;
}

if (fileCount === 0) {
  throw new Error('No generated TypeScript bindings found');
}
