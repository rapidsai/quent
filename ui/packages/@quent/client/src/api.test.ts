// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, afterEach } from 'vitest';
import type { DataFlowTimelineBinned, TimelineConfig } from '@quent/utils';
import {
  createHttpApiClient,
  fetchDataFlow,
  fetchListEngines,
  fetchListQueries,
} from './api';

const CONFIG: TimelineConfig = { start: 0, end: 8, num_bins: 4 };

const BINNED: DataFlowTimelineBinned = {
  config: { span: { start: 0, end: 8 }, bin_duration: 2, num_bins: 4 as unknown as bigint },
  decl: {
    entity_type_name: 'Task',
    dimension_name: 'Data location',
    dimension_keys: [{ key: 'memory', display_name: 'Memory' }],
    measures: [{ name: 'tasks', display_name: 'Tasks', quantity: 'unit', kind: 'Occupancy' }],
    default_measure: null,
  },
  operators: {
    'op-1': { values: { tasks: { queueing: { memory: [1, 2, 0, 0] } } } },
  },
};

function stubFetch(response: Response) {
  const fetchMock = vi.fn().mockResolvedValue(response);
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

describe('catalog fetchers', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('fetches the engine catalog with metadata', async () => {
    const response = { items: [], initial_group_by_attribute: 'frontend' };
    const fetchMock = stubFetch(new Response(JSON.stringify(response), { status: 200 }));

    await expect(fetchListEngines()).resolves.toEqual(response);
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringMatching(/\/engines\?with_metadata=true$/),
      expect.any(Object)
    );
  });

  it('fetches every query for an engine without a query-group parameter', async () => {
    const response = { items: [], initial_group_by_attribute: 'workload' };
    const fetchMock = stubFetch(new Response(JSON.stringify(response), { status: 200 }));

    await expect(fetchListQueries('engine-1')).resolves.toEqual(response);
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringMatching(/\/engines\/engine-1\/queries$/),
      expect.any(Object)
    );
  });

  it('does not treat a TanStack Query callback context as the API base URL', async () => {
    const response = { items: [], initial_group_by_attribute: null };
    const fetchMock = stubFetch(new Response(JSON.stringify(response), { status: 200 }));
    const queryFunction = fetchListEngines as unknown as (
      context: object
    ) => ReturnType<typeof fetchListEngines>;

    await queryFunction({ queryKey: ['list_engines'] });

    expect(fetchMock.mock.calls[0]?.[0]).toMatch(/\/api\/engines\?with_metadata=true$/u);
    expect(fetchMock.mock.calls[0]?.[0]).not.toContain('[object Object]');
  });
});

describe('fetchDataFlow', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('resolves the binned timeline on a 200 response', async () => {
    stubFetch(new Response(JSON.stringify(BINNED), { status: 200 }));
    await expect(fetchDataFlow('e-1', 'q-1', CONFIG)).resolves.toEqual(BINNED);
  });

  it('resolves to the null sentinel on HTTP 501 (unsupported analyzer)', async () => {
    stubFetch(new Response(null, { status: 501, statusText: 'Not Implemented' }));
    await expect(fetchDataFlow('e-1', 'q-1', CONFIG)).resolves.toBeNull();
  });

  it('still rejects on other non-ok statuses', async () => {
    stubFetch(new Response(null, { status: 500, statusText: 'Internal Server Error' }));
    await expect(fetchDataFlow('e-1', 'q-1', CONFIG)).rejects.toThrow(
      'API Error: 500 Internal Server Error'
    );
  });

  it('POSTs a CategoricalTimelineRequest to the data-flow endpoint', async () => {
    const fetchMock = stubFetch(new Response(JSON.stringify(BINNED), { status: 200 }));
    await fetchDataFlow('e-1', 'q-1', CONFIG, ['tasks']);
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/engines/e-1/timeline/data-flow'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          measures: ['tasks'],
          config: CONFIG,
          app_params: { query_id: 'q-1' },
        }),
      })
    );
  });
});

describe('createHttpApiClient', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('keeps concurrent clients pinned to independent API bases', async () => {
    const fetchMock = vi.fn().mockImplementation(async () => {
      return new Response(
        JSON.stringify({ items: [], initial_group_by_attribute: null }),
        { status: 200 }
      );
    });
    vi.stubGlobal('fetch', fetchMock);
    const first = createHttpApiClient('http://first.test/api/');
    const second = createHttpApiClient('http://second.test/api');

    await Promise.all([first.fetchListEngines(), second.fetchListEngines()]);

    expect(fetchMock.mock.calls.map(call => call[0])).toEqual([
      'http://first.test/api/engines?with_metadata=true',
      'http://second.test/api/engines?with_metadata=true',
    ]);
  });
});
