// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { PassThrough } from 'node:stream';
import { describe, expect, it, vi } from 'vitest';
import type { ApiClient } from '@quent/client';
import type { EntityRef, QueryBundle } from '@quent/utils';
import {
  createNonInteractiveSelector,
  createTerminalSelector,
  querySelectionKey,
  resolveAskSelections,
  selectQueriesFromTree,
  type SelectFromList,
  type SelectFromQueryTree,
} from './askSelection';

const queryBundle = {
  query_id: 'query-2',
  duration_s: 10,
  entities: {
    resources: {
      'resource-2': {
        id: 'resource-2',
        instance_name: 'GPU Memory',
        type_name: 'Memory',
        parent_group_id: 'gpu',
      },
      'resource-1': {
        id: 'resource-1',
        instance_name: 'Host Memory',
        type_name: 'Memory',
        parent_group_id: 'worker',
      },
    },
  },
} as unknown as QueryBundle<EntityRef>;

function discoveryApi() {
  return {
    fetchListEngines: vi.fn().mockResolvedValue({
      items: [
        {
          id: 'engine-2',
          instance_name: 'Zeta Engine',
          start_time_unix_ns: null,
          duration_s: null,
          custom_attributes: [],
          implementation: null,
        },
        {
          id: 'engine-1',
          instance_name: 'Alpha Engine',
          start_time_unix_ns: null,
          duration_s: null,
          custom_attributes: [],
          implementation: null,
        },
      ],
      initial_group_by_attribute: null,
    }),
    fetchListQueries: vi.fn().mockResolvedValue({
      items: [
        {
          id: 'query-2',
          instance_name: 'Query Z',
          custom_attributes: [{ key: 'workload', value: { String: 'Warehouse' } }],
          start_unix_ns: null,
          planning_s: null,
          executing_s: null,
          completed_s: null,
        },
        {
          id: 'query-1',
          instance_name: 'Query A',
          custom_attributes: [{ key: 'workload', value: { String: 'Ad hoc' } }],
          start_unix_ns: null,
          planning_s: null,
          executing_s: null,
          completed_s: null,
        },
      ],
      initial_group_by_attribute: 'workload',
    }),
    fetchQueryBundle: vi.fn().mockResolvedValue(queryBundle),
  } satisfies Pick<ApiClient, 'fetchListEngines' | 'fetchListQueries' | 'fetchQueryBundle'>;
}

describe('ask CLI selections', () => {
  it('discovers and presents missing engine, query, and resource IDs', async () => {
    const api = discoveryApi();
    const select: SelectFromList = vi.fn(async prompt => {
      expect(prompt).toBe('Select a resource');
      return 'resource-2';
    });
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValue([querySelectionKey('engine-2', 'query-2')]);

    const resolved = await resolveAskSelections({
      values: {},
      api,
      select,
      selectTree,
      requireResource: true,
    });

    expect(api.fetchListQueries).toHaveBeenCalledWith('engine-1');
    expect(api.fetchListQueries).toHaveBeenCalledWith('engine-2');
    expect(api.fetchQueryBundle).toHaveBeenCalledWith('engine-2', 'query-2');
    expect(selectTree).toHaveBeenCalledWith(
      'Select a query',
      expect.arrayContaining([
        expect.objectContaining({
          engineId: 'engine-2',
          engineLabel: 'Zeta Engine (engine-2)',
          queryId: 'query-2',
        }),
      ]),
      false
    );
    expect(select).toHaveBeenCalledWith('Select a resource', [
      { value: 'resource-2', label: 'GPU Memory (Memory, resource-2)' },
      { value: 'resource-1', label: 'Host Memory (Memory, resource-1)' },
    ]);
    expect(resolved).toMatchObject({
      engineId: 'engine-2',
      queryId: 'query-2',
      queryBundle,
      values: {
        engine: 'engine-2',
        query: 'query-2',
        resource: 'resource-2',
      },
    });
  });

  it('does not issue list requests for IDs supplied by the caller', async () => {
    const api = discoveryApi();
    const select = vi.fn<SelectFromList>();
    const selectTree = vi.fn<SelectFromQueryTree>();

    const resolved = await resolveAskSelections({
      values: {
        engine: ' engine-2 ',
        query: ' query-2 ',
        resource: ' resource-1 ',
      },
      api,
      select,
      selectTree,
      requireResource: true,
    });

    expect(api.fetchListEngines).not.toHaveBeenCalled();
    expect(api.fetchListQueries).not.toHaveBeenCalled();
    expect(select).not.toHaveBeenCalled();
    expect(selectTree).not.toHaveBeenCalled();
    expect(api.fetchQueryBundle).toHaveBeenCalledWith('engine-2', 'query-2');
    expect(resolved.values).toMatchObject({
      engine: 'engine-2',
      query: 'query-2',
      resource: 'resource-1',
    });
  });

  it('scopes the query tree when only the engine ID is supplied', async () => {
    const api = discoveryApi();
    const select = vi.fn<SelectFromList>();
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValue([querySelectionKey('engine-2', 'query-2')]);

    await expect(
      resolveAskSelections({
        values: { engine: 'engine-2' },
        api,
        select,
        selectTree,
        requireResource: false,
      })
    ).resolves.toMatchObject({
      engineId: 'engine-2',
      queryId: 'query-2',
    });

    expect(api.fetchListQueries).toHaveBeenCalledTimes(1);
    expect(api.fetchListQueries).toHaveBeenCalledWith('engine-2');
    expect(selectTree).toHaveBeenCalledWith(
      'Select a query',
      expect.arrayContaining([
        expect.objectContaining({ engineId: 'engine-2', queryId: 'query-2' }),
      ]),
      false
    );
  });

  it('builds an all-engine query tree and returns multiple selections', async () => {
    const api = discoveryApi();
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValue([
        querySelectionKey('engine-2', 'query-1'),
        querySelectionKey('engine-2', 'query-2'),
      ]);

    await expect(
      selectQueriesFromTree(api, selectTree, 'Select candidates', true)
    ).resolves.toEqual([
      { engineId: 'engine-2', queryId: 'query-1' },
      { engineId: 'engine-2', queryId: 'query-2' },
    ]);
    expect(selectTree).toHaveBeenCalledWith(
      'Select candidates',
      expect.arrayContaining([
        expect.objectContaining({
          engineId: 'engine-2',
          engineLabel: 'Zeta Engine (engine-2)',
          queryId: 'query-2',
        }),
      ]),
      true
    );
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
