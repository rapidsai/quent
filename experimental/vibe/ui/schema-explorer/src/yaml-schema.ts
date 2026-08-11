// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Schema } from '@quent/schema-viewer';

import initialize, {
  parse_schema_json,
} from '../wasm/quent_yaml.js';

let initialization: Promise<unknown> | null = null;

export async function parseYamlSchema(source: string): Promise<Schema> {
  initialization ??= initialize();
  await initialization;
  return JSON.parse(parse_schema_json(source)) as Schema;
}

export function parserErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
