// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import type { NvtxCatalog, NvtxViewportRequest } from '@quent/utils';
import { fetchNvtxSinks, fetchNvtxCatalog, fetchNvtxViewport } from './api';
import { canonicalizeNvtxRequest, nvtxViewportQueryOptions, selectNvtxDomains } from './nvtx';

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

function nvtxDomain(
  domainId: string,
  categoryIds: number[],
  hasUncategorized: boolean,
  sourceDomainIds: string[] = [domainId]
): NvtxCatalog['domains'][number] {
  return {
    domain_id: domainId,
    source_domain_ids: sourceDomainIds,
    name: `domain ${domainId}`,
    color: '#000000ff',
    threads: [],
    categories: categoryIds.map(categoryId => ({
      category_id: categoryId,
      name: `category ${categoryId}`,
    })),
    has_uncategorized: hasUncategorized,
  };
}

describe('NVTX client', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('discovers sink entities through application analysis', async () => {
    const fetchMock = stubFetch(new Response('["sink-1","sink-2"]', { status: 200 }));
    await expect(fetchNvtxSinks('engine-1')).resolves.toEqual(['sink-1', 'sink-2']);
    expect(fetchMock.mock.calls[0]?.[0]).toContain('/api/engines/engine-1/nvtx');
  });

  it('treats catalog 404 as optional absence', async () => {
    const fetchMock = stubFetch(new Response(null, { status: 404, statusText: 'Not Found' }));
    await expect(fetchNvtxCatalog('engine-1', 'sink-1', QUERY_START_UNIX_NS)).resolves.toBeNull();
    expect(fetchMock.mock.calls[0]?.[0]).toContain(
      `/api/engines/engine-1/nvtx/sink-1/catalog?query_start=${QUERY_START_UNIX_NS}`
    );
  });

  it('propagates non-404 catalog failures', async () => {
    stubFetch(new Response(null, { status: 500, statusText: 'Internal Server Error' }));
    await expect(fetchNvtxCatalog('engine-1', 'sink-1', QUERY_START_UNIX_NS)).rejects.toThrow(
      'API Error: 500 Internal Server Error'
    );
  });

  it('treats viewport 404 as optional absence and propagates other failures', async () => {
    stubFetch(new Response(null, { status: 404, statusText: 'Not Found' }));
    await expect(
      fetchNvtxViewport('engine-1', 'sink-1', QUERY_START_UNIX_NS, request)
    ).resolves.toBeNull();

    stubFetch(new Response(null, { status: 500, statusText: 'Internal Server Error' }));
    await expect(
      fetchNvtxViewport('engine-1', 'sink-1', QUERY_START_UNIX_NS, request)
    ).rejects.toThrow('API Error: 500 Internal Server Error');
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

    await fetchNvtxViewport('engine-1', 'sink-1', QUERY_START_UNIX_NS, request);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toContain(
      `/api/engines/engine-1/nvtx/sink-1/viewport?query_start=${QUERY_START_UNIX_NS}`
    );
    expect(JSON.parse(init.body as string)).toEqual(canonical);

    const invalid = { ...request, viewport: { start: 2, end: 1 } };
    expect(() =>
      nvtxViewportQueryOptions('engine-1', 'sink-1', QUERY_START_UNIX_NS, invalid)
    ).not.toThrow();
    await expect(
      fetchNvtxViewport('engine-1', 'sink-1', QUERY_START_UNIX_NS, invalid)
    ).rejects.toThrow('NVTX viewport bounds must be finite and ordered');
  });

  it('selects NVTX domains and categories', () => {
    const catalog = {
      domains: [nvtxDomain('1', [7], true), nvtxDomain('3', [], true)],
    } satisfies Pick<NvtxCatalog, 'domains'>;

    expect(selectNvtxDomains(catalog, '3').map(selection => selection.domain_id)).toEqual(['3']);
    expect(selectNvtxDomains(catalog, null).map(selection => selection.domain_id)).toEqual([
      '1',
      '3',
    ]);
    expect(
      selectNvtxDomains(
        catalog,
        '1',
        new Map([['1', { categoryId: 7, includeUncategorized: false }]])
      )
    ).toEqual([{ domain_id: '1', category_ids: [7], include_uncategorized: false }]);
    expect(
      selectNvtxDomains(
        catalog,
        '1',
        new Map([['1', { categoryId: null, includeUncategorized: true }]])
      )
    ).toEqual([{ domain_id: '1', category_ids: [], include_uncategorized: true }]);
    expect(
      selectNvtxDomains(
        catalog,
        '1',
        new Map([['1', { categoryId: 8, includeUncategorized: false }]])
      )
    ).toEqual([]);
    expect(
      selectNvtxDomains(
        catalog,
        null,
        new Map([['1', { categoryId: 7, includeUncategorized: false }]])
      )
    ).toEqual([
      { domain_id: '1', category_ids: [7], include_uncategorized: false },
      { domain_id: '3', category_ids: [], include_uncategorized: true },
    ]);

    const grouped = {
      domains: [nvtxDomain('5', [7], false, ['5', '172'])],
    } satisfies Pick<NvtxCatalog, 'domains'>;
    expect(selectNvtxDomains(grouped, null)).toEqual([
      { domain_id: '5', category_ids: [7], include_uncategorized: false },
    ]);
  });

  it('normalizes even safe u64 statistic counts to bigint', async () => {
    stubFetch(
      new Response(
        '{"viewport":{"start":-0.25,"end":1.5},"domains":[],"statistics":[{"message":"work","domain_id":"3","source_domain_ids":["3"],"domain_name":"d","category_id":null,"category_name":null,"count":1,"observed_count":1,"total_duration":1.25,"avg_duration":1.25,"min_duration":1.25,"max_duration":1.25,"saturated":false}]}',
        { status: 200 }
      )
    );
    const viewport = await fetchNvtxViewport('engine-1', 'sink-1', QUERY_START_UNIX_NS, request);
    expect(viewport?.statistics[0]?.count).toBe(1n);
    expect(viewport?.statistics[0]?.observed_count).toBe(1n);
    expect(viewport?.statistics[0]?.total_duration).toBe(1.25);
  });
});
