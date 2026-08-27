// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import {
  initSync,
  parse_schema_json,
} from '../wasm/quent_yaml.js';

const wasm = await readFile(
  new URL('../wasm/quent_yaml_bg.wasm', import.meta.url),
);
initSync({ module: wasm });

const examples = [
  ['simple', 3],
  ['hello', 1],
  ['dynamo-inference', 23],
  ['simulator', 14],
  ['sirius', 16],
];

for (const [name, entityCount] of examples) {
  const source = await readFile(
    new URL(`../src/models/${name}.yaml`, import.meta.url),
    'utf8',
  );
  const schema = JSON.parse(parse_schema_json(source));
  assert.equal(schema.entities.length, entityCount, name);
  assert.equal(typeof schema.entities[0][0].name, 'string', name);
  assert.ok(Array.isArray(schema.entities[0][0].namespace), name);
}

assert.throws(
  () => parse_schema_json('quent: alpha\nmodel: broken\nentities: ['),
  /editor\.yaml/,
);
