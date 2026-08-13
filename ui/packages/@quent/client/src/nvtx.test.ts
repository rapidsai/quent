// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import type { NvtxViewportRequest } from '@quent/utils';
import { fetchNvtxCatalog, fetchNvtxViewport } from './api';
import {
  canonicalizeNvtxRequest,
  nvtxCatalogQueryOptions,
  nvtxCatalogStaleTime,
  nvtxViewportQueryOptions,
} from './nvtx';

function stubFetch(response: Response) {
  const fetchMock = vi.fn().mockResolvedValue(response);
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

const QUERY_START_UNIX_NS = 1_800_000_000_000_000_001n;

const request: NvtxViewportRequest = {
  viewport: { start: -0.25, end: 1.5 },
  selections: [
    { domain_id: '9', category_ids: [7, 3, 7], include_uncategorized: false },
    { domain_id: '2', category_ids: [], include_uncategorized: true },
  ],
};

describe('NVTX client', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('treats catalog 404 as optional absence and keeps absent streams retryable', async () => {
    const fetchMock = stubFetch(new Response(null, { status: 404, statusText: 'Not Found' }));
    await expect(fetchNvtxCatalog('context-1', QUERY_START_UNIX_NS)).resolves.toBeNull();
    expect(fetchMock.mock.calls[0]?.[0]).toContain(
      `/api/nvtx/contexts/context-1/catalog?query_start=${QUERY_START_UNIX_NS}`
    );
    expect(nvtxCatalogQueryOptions('context-1', QUERY_START_UNIX_NS).staleTime).toBeTypeOf(
      'function'
    );
    expect(nvtxCatalogStaleTime(null)).toBe(0);
    expect(nvtxCatalogStaleTime(undefined)).toBe(Infinity);
  });

  it('propagates non-404 catalog failures', async () => {
    stubFetch(new Response(null, { status: 500, statusText: 'Internal Server Error' }));
    await expect(fetchNvtxCatalog('context-1', QUERY_START_UNIX_NS)).rejects.toThrow(
      'API Error: 500 Internal Server Error'
    );
  });

  it('canonicalizes at the fetch boundary without validating during render', async () => {
    const fetchMock = stubFetch(
      new Response('{"viewport":{"start":-0.25,"end":1.5},"domains":[],"statistics":[]}', {
        status: 200,
      })
    );
    const canonical = canonicalizeNvtxRequest(request);
    expect(canonical.selections.map(selection => selection.domain_id)).toEqual(['2', '9']);
    expect(canonical.selections[1].category_ids).toEqual([3, 7]);

    await fetchNvtxViewport('context-1', QUERY_START_UNIX_NS, request);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toContain(
      `/api/nvtx/contexts/context-1/viewport?query_start=${QUERY_START_UNIX_NS}`
    );
    expect(init.body).toBe(
      '{"viewport":{"start":-0.25,"end":1.5},"selections":[{"domain_id":"2","category_ids":[],"include_uncategorized":true},{"domain_id":"9","category_ids":[3,7],"include_uncategorized":false}]}'
    );

    const options = nvtxViewportQueryOptions('context-1', QUERY_START_UNIX_NS, request);
    expect(options.queryKey).toEqual([
      'nvtxViewport',
      'context-1',
      QUERY_START_UNIX_NS.toString(10),
      -0.25,
      1.5,
      [
        ['9', [7, 3, 7], false],
        ['2', [], true],
      ],
    ]);
    expect(options.placeholderData).toBeTypeOf('function');

    const invalid = { ...request, viewport: { start: 2, end: 1 } };
    expect(() => nvtxViewportQueryOptions('context-1', QUERY_START_UNIX_NS, invalid)).not.toThrow();
    await expect(fetchNvtxViewport('context-1', QUERY_START_UNIX_NS, invalid)).rejects.toThrow(
      'NVTX viewport bounds must be finite and ordered'
    );
  });

  it('preserves relative seconds and decimal-string identifiers from the catalog', async () => {
    stubFetch(
      new Response(
        '{"trace_start":-0.5,"trace_end":2,"domains":[{"domain_id":"3","name":"d","color":"#000000","threads":[],"categories":[],"has_uncategorized":false}],"anomalies":{"orphan_range_ends":"0","orphan_range_pops":"0","orphan_resource_destroys":"0","reused_range_ids":"0","reused_resource_handles":"0","total":"0","is_faithful":true}}',
        { status: 200 }
      )
    );
    const catalog = await fetchNvtxCatalog('context-1', QUERY_START_UNIX_NS);
    expect(catalog?.trace_start).toBe(-0.5);
    expect(catalog?.domains[0].domain_id).toBe('3');
    expect(catalog?.anomalies.total).toBe('0');
  });

  it('normalizes even safe u64 statistic counts to bigint', async () => {
    stubFetch(
      new Response(
        '{"viewport":{"start":-0.25,"end":1.5},"domains":[],"statistics":[{"message":"work","domain_id":"3","domain_name":"d","category_id":null,"category_name":null,"count":1,"observed_count":1,"total_duration":1.25,"avg_duration":1.25,"min_duration":1.25,"max_duration":1.25,"saturated":false}]}',
        { status: 200 }
      )
    );
    const viewport = await fetchNvtxViewport('context-1', QUERY_START_UNIX_NS, request);
    expect(viewport.statistics[0]?.count).toBe(1n);
    expect(viewport.statistics[0]?.observed_count).toBe(1n);
    expect(viewport.statistics[0]?.total_duration).toBe(1.25);
  });
});
