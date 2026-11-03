// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { PassThrough } from 'node:stream';
import { describe, expect, it, vi } from 'vitest';
import type { ApiClient } from '@quent/client';
import {
  createNonInteractiveSelector,
  createTerminalSelector,
  selectEngine,
  type DiscoveryApi,
  type SelectFromList,
} from './askSelection';

function discoveryApi(): DiscoveryApi {
  return {
    fetchListEngines: vi.fn().mockResolvedValue({
      items: [
        { id: 'engine-2', instance_name: 'Zeta Engine' },
        { id: 'engine-1', instance_name: 'Alpha Engine' },
      ],
      initial_group_by_attribute: null,
    }),
    fetchListQueries: vi.fn().mockResolvedValue({
      items: [
        {
          id: 'query-2',
          instance_name: 'Query Z',
          custom_attributes: [{ key: 'workload', value: { String: 'Warehouse' } }],
        },
        {
          id: 'query-1',
          instance_name: 'Query A',
          custom_attributes: [{ key: 'workload', value: { String: 'Ad hoc' } }],
        },
      ],
      initial_group_by_attribute: 'workload',
    }),
  } as unknown as Pick<ApiClient, 'fetchListEngines' | 'fetchListQueries'>;
}

describe('ask CLI selections', () => {
  it('selects engines in stable label order', async () => {
    const api = discoveryApi();
    const select = vi.fn<SelectFromList>().mockResolvedValue('engine-1');

    await expect(selectEngine(api, select)).resolves.toBe('engine-1');
    expect(select).toHaveBeenCalledWith('Select an engine', [
      { value: 'engine-1', label: 'Alpha Engine (engine-1)' },
      { value: 'engine-2', label: 'Zeta Engine (engine-2)' },
    ]);
  });

  it('fails clearly when interactive selection is unavailable', async () => {
    const input = Object.assign(new PassThrough(), { isTTY: false });
    const output = Object.assign(new PassThrough(), { isTTY: false });
    const select = createTerminalSelector(input, output);

    await expect(
      select('Select an engine', [{ value: 'engine-1', label: 'Engine 1' }])
    ).rejects.toThrow('requires an interactive terminal');
  });

  it('never opens a prompt in JSON mode', async () => {
    const select = createNonInteractiveSelector();

    await expect(select('Select an engine', [])).rejects.toThrow(
      'Select an engine is required in JSON mode'
    );
  });
});
