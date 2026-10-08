// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { inlineSinglePage } from '../scripts/inline.mjs';

test('single-page inlining preserves JavaScript replacement tokens', () => {
  const html = '<html><head><script type="module" crossorigin src="./assets/app.js"></script>' +
    '<link rel="stylesheet" crossorigin href="./assets/app.css"></head><body></body></html>';
  const javascript = 'const value = "$& $` $\'";';
  const report = JSON.stringify({ name: '</script><script>alert(1)</script>' });
  const result = inlineSinglePage(html, javascript, 'body { color: red; }', report);
  assert.ok(result.includes(`<script type="module">${javascript}</script>`));
  assert.ok(result.includes('<style>body { color: red; }</style>'));
  assert.doesNotMatch(result, /(?:src|href)="\.\/assets\//);
  const embedded = result.match(/<script id="quent-bench-report" type="application\/json">(.*?)<\/script>/)?.[1];
  assert.equal(JSON.parse(embedded).name, '</script><script>alert(1)</script>');
});
