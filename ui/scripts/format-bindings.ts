// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { glob, readFile, writeFile } from 'node:fs/promises';
import { format } from 'vite-plus/fmt';

const patterns = process.argv.slice(2);
const fileNames = new Set<string>();

for (const pattern of patterns.length > 0 ? patterns : ['generated/ts-bindings/**/*.ts']) {
  for await (const fileName of glob(pattern)) {
    fileNames.add(fileName);
  }
}

for (const fileName of fileNames) {
  const sourceText = await readFile(fileName, 'utf8');
  const result = await format(fileName, sourceText, {
    semi: true,
    trailingComma: 'es5',
    singleQuote: true,
    printWidth: 100,
    tabWidth: 2,
    useTabs: false,
    arrowParens: 'avoid',
    endOfLine: 'lf',
  });

  if (result.errors.length > 0) {
    throw new Error(
      `Failed to format ${fileName}: ${result.errors.map(error => error.message).join('; ')}`
    );
  }

  if (result.code !== sourceText) {
    await writeFile(fileName, result.code);
  }
}

if (fileNames.size === 0) {
  throw new Error('No generated TypeScript bindings found');
}
