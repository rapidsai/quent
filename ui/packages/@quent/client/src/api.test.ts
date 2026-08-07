// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, afterEach } from 'vitest';
import type { DataFlowTimelineBinned, TimelineConfig } from '@quent/utils';
import { fetchDataFlow } from './api';

const CONFIG: TimelineConfig = { start: 0, end: 8, num_bins: 4 };

const BINNED: DataFlowTimelineBinned = {
  config: { span: { start: 0, end: 8 }, bin_duration: 2, num_bins: 4 },
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
