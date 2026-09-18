// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { Worker } from './types';
import { workerDisplayName } from './worker';

function makeWorker(instanceName: string | null): Worker {
  return {
    id: 'worker-1',
    parent_engine_id: null,
    instance_name: instanceName,
    start_unix_ns: null,
    end_unix_ns: null,
  };
}

describe('workerDisplayName', () => {
  it('uses the instance name when present', () => {
    expect(workerDisplayName(makeWorker('GPU Worker'))).toBe('GPU Worker');
  });

  it('falls back to the worker ID', () => {
    expect(workerDisplayName(makeWorker(null))).toBe('worker-1');
  });
});
