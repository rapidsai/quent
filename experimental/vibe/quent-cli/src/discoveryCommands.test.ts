// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { getApiClient, setApiClient, type ApiClient } from '@quent/client';
import { enginesCommand, queriesCommand } from './discoveryCommands';

const originalClient = getApiClient();

afterEach(() => {
  setApiClient(originalClient);
  vi.restoreAllMocks();
});

function captureStdout() {
  return vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
}

describe('agent discovery commands', () => {
  it('lists engines in a versioned, deterministic JSON envelope', async () => {
    setApiClient({
      fetchListEngines: vi.fn().mockResolvedValue({
        items: [
          { id: 'engine-z', instance_name: 'Zeta' },
          { id: 'engine-a', instance_name: 'Alpha' },
        ],
        initial_group_by_attribute: null,
      }),
    } as unknown as ApiClient);
    const write = captureStdout();

    await enginesCommand.run(['--json']);

    expect(JSON.parse(String(write.mock.calls[0]![0]))).toEqual({
      schemaVersion: 2,
      command: 'engines',
      data: [
        { id: 'engine-a', instance_name: 'Alpha' },
        { id: 'engine-z', instance_name: 'Zeta' },
      ],
    });
  });

  it('lists every query for an engine', async () => {
    setApiClient({
      fetchListQueries: vi.fn().mockResolvedValue({
        items: [
          {
            id: 'query-1',
            instance_name: 'Daily report',
            custom_attributes: [{ key: 'workload', value: { String: 'Warehouse' } }],
          },
        ],
        initial_group_by_attribute: 'workload',
      }),
    } as unknown as ApiClient);
    const write = captureStdout();

    await queriesCommand.run(['--engine', 'engine-1', '--json']);

    expect(JSON.parse(String(write.mock.calls[0]![0])).data).toEqual({
      engineId: 'engine-1',
      queries: [
        {
          id: 'query-1',
          instance_name: 'Daily report',
          custom_attributes: [{ key: 'workload', value: { String: 'Warehouse' } }],
        },
      ],
    });
  });

  it('requires an explicit engine before issuing a JSON query request', async () => {
    setApiClient({
      fetchListEngines: vi.fn(),
      fetchListQueries: vi.fn(),
    } as unknown as ApiClient);

    await expect(queriesCommand.run(['--json'])).rejects.toThrow(
      'JSON mode requires explicit --engine'
    );
    expect(getApiClient().fetchListEngines).not.toHaveBeenCalled();
    expect(getApiClient().fetchListQueries).not.toHaveBeenCalled();
  });
});
