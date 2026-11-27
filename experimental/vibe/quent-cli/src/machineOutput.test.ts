// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  ASK_OUTPUT_SCHEMA_VERSION,
  machineError,
  machineResult,
  serializeMachineOutput,
} from './machineOutput';

describe('ask machine output', () => {
  it('wraps successful data in a versioned command envelope', () => {
    expect(machineResult('engines', [{ id: 'engine-1' }])).toEqual({
      schemaVersion: ASK_OUTPUT_SCHEMA_VERSION,
      command: 'engines',
      data: [{ id: 'engine-1' }],
    });
  });

  it('serializes API bigint fields as decimal strings', () => {
    const output = serializeMachineOutput(
      machineResult('engines', [{ id: 'engine-1', start_time_unix_ns: 42n }])
    );

    expect(JSON.parse(output).data[0].start_time_unix_ns).toBe('42');
  });

  it('serializes errors without usage or terminal formatting', () => {
    const output = serializeMachineOutput(machineError('query-diff', new Error('Missing ID')));

    expect(JSON.parse(output)).toEqual({
      schemaVersion: ASK_OUTPUT_SCHEMA_VERSION,
      command: 'query-diff',
      error: { message: 'Missing ID' },
    });
    expect(output).not.toContain('\u001B[');
  });
});
