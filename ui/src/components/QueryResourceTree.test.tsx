// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { waitFor, act } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderWithQuery } from '@/test/test-utils';
import { server } from '@/test/mocks/server';
import { Provider as JotaiProvider, createStore } from 'jotai';
import { QueryResourceTree } from './QueryResourceTree';
import { applyBulkTimelineResponse, timelineCacheKey } from '@quent/hooks';
import { timelineDataMapAtom } from '@quent/hooks/testing';
import type {
  SingleTimelineResponse,
  QueryBundle,
  EntityRef,
  NvtxCatalog,
  NvtxViewportResponse,
} from '@quent/utils';

// ---------------------------------------------------------------------------
// Mock heavy/visual dependencies so tests run without a real browser/canvas
// ---------------------------------------------------------------------------

vi.mock('@quent/hooks', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/hooks')>();
  return {
    ...actual,
    useBulkTimelines: () => ({ handleZoomChange: vi.fn(), handleExpand: vi.fn() }),
    useHighlightedItemIds: () => new Set<string>(),
  };
});

vi.mock('@/hooks/useExpandedIds', () => ({
  useExpandedIds: () => ({ expandedIds: new Set<string>(), handleExpandChange: vi.fn() }),
}));

vi.mock('@/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'light', setTheme: vi.fn() }),
  THEME_DARK: 'dark',
  THEME_LIGHT: 'light',
}));

// Capture the timelineData prop passed to TimelineController on every render
let capturedTimelineData: SingleTimelineResponse | null | undefined = undefined;
interface CapturedTreeItem {
  id: string;
  children?: CapturedTreeItem[];
}

let capturedTreeData: CapturedTreeItem[] = [];

function findCapturedItem(id: string, items = capturedTreeData): CapturedTreeItem | undefined {
  for (const item of items) {
    if (item.id === id) return item;
    const child = item.children ? findCapturedItem(id, item.children) : undefined;
    if (child) return child;
  }
  return undefined;
}

// Mock @quent/components: keep all actual exports but override heavy/visual ones
vi.mock('@quent/components', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/components')>();
  return {
    ...actual,
    TimelineController: (props: { timelineData?: SingleTimelineResponse | null }) => {
      capturedTimelineData = props.timelineData;
      return null;
    },
    TreeTable: ({
      columns,
      data,
    }: {
      columns: Array<{ headerContent?: React.ReactNode; subHeaderContent?: React.ReactNode }>;
      data: CapturedTreeItem[];
    }) => {
      capturedTreeData = data;
      return (
        <>
          {columns.map((col, i) => (
            <React.Fragment key={i}>
              {col.headerContent}
              {col.subHeaderContent}
            </React.Fragment>
          ))}
        </>
      );
    },
    ResourceColumn: () => null,
    UsageColumn: () => null,
    TimelineToolbar: () => null,
  };
});

import * as clientApi from '@quent/client';
vi.mock('@quent/client', async importOriginal => {
  const actual = await importOriginal<typeof clientApi>();
  return {
    ...actual,
    fetchSingleTimeline: vi.fn(),
    fetchBulkTimelines: vi.fn(),
    fetchEngineContexts: vi.fn(),
    fetchNvtxCatalog: vi.fn(),
    fetchNvtxViewport: vi.fn(),
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DURATION_S = 100;
const ROOT_GROUP_ID = 'qg-1';
const RESOURCE_ID = 'res-1';
const RESOURCE_TYPE = 'GPU';

/** Minimal QueryBundle that causes the root timeline query to be enabled. */
const makeBundle = (workerId?: string): QueryBundle<EntityRef> => {
  const resource = { Resource: { Resource: RESOURCE_ID } };
  return {
    query_id: 'test-query',
    entities: {
      engine: { id: 'engine-1' },
      query_group: { id: ROOT_GROUP_ID },
      query: { id: 'query-1' },
      workers: workerId ? { [workerId]: { id: workerId } } : {},
      plans: {},
      operators: {},
      ports: {},
      resource_types: { [RESOURCE_TYPE]: { used_by: ['task'], capacities: [] } },
      resource_group_types: {},
      resources: { [RESOURCE_ID]: { id: RESOURCE_ID, type_name: RESOURCE_TYPE } },
      resource_groups: {},
      fsm_types: {},
    },
    resource_tree: {
      ResourceGroup: {
        id: { QueryGroup: ROOT_GROUP_ID },
        children: workerId
          ? [{ ResourceGroup: { id: { Worker: workerId }, children: [resource] } }]
          : [resource],
      },
    },
    plan_tree: { id: 'plan-1', worker: workerId ?? null, children: [] },
    unique_operator_names: [],
    quantity_specs: {},
    start_time_unix_ns: 0n,
    duration_s: DURATION_S,
  } as unknown as QueryBundle<EntityRef>;
};

const makeTimeline = (start: number, end: number): SingleTimelineResponse =>
  ({
    config: { span: { start, end }, bin_duration: 1, num_bins: BigInt(end - start) },
    data: { Binned: { series: {} } },
  }) as unknown as SingleTimelineResponse;

describe('QueryResourceTree — TimelineController always shows full-range data', () => {
  beforeEach(() => {
    capturedTimelineData = undefined;
    capturedTreeData = [];
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  it('passes full-range timeline data to TimelineController', async () => {
    const fullRange = makeTimeline(0, DURATION_S);
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(fullRange);

    const store = createStore();
    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await waitFor(() => expect(capturedTimelineData).toBe(fullRange));
    expect(capturedTimelineData?.config.span.start).toBe(0);
    expect(capturedTimelineData?.config.span.end).toBe(DURATION_S);
  });

  it('is unaffected when a zoom-bounded bulk fetch overwrites the same atom cache key', async () => {
    const fullRange = makeTimeline(0, DURATION_S);
    const zoomed = makeTimeline(25, 75);
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(fullRange);

    const store = createStore();
    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    // Wait for the full-range data to appear in TimelineController
    await waitFor(() => expect(capturedTimelineData).toBe(fullRange));

    // Simulate what useBulkTimelines does when the user zooms: it calls
    // applyBulkTimelineResponse which writes zoom-bounded data to timelineDataAtom
    // under the same key that was previously used for the full-range data.
    // Wrap in act() so any atom-subscription re-renders are flushed synchronously
    // before we assert — this is what makes the test fail on the buggy code.
    const idToMeta = new Map([
      [
        'bulk-id-1',
        {
          resourceId: ROOT_GROUP_ID,
          resourceTypeName: RESOURCE_TYPE,
          operatorId: null,
          fsmTypeName: null,
        },
      ],
    ]);
    await act(async () => {
      applyBulkTimelineResponse(
        {
          entries: {
            'bulk-id-1': { status: 'ok', data: zoomed.data, config: zoomed.config } as never,
          },
        },
        idToMeta,
        store
      );
    });

    // Confirm the atom was indeed overwritten with zoomed data (bug mechanism is intact)
    const cacheKey = timelineCacheKey({
      resourceId: ROOT_GROUP_ID,
      resourceTypeName: RESOURCE_TYPE,
      fsmTypeName: null,
    });
    const timelineMap = store.get(timelineDataMapAtom) as Record<string, SingleTimelineResponse>;
    expect(timelineMap[cacheKey]?.config.span.start).toBe(25);

    // TimelineController must still show the full-range data — not the atom value.
    expect(capturedTimelineData?.config.span.start).toBe(0);
    expect(capturedTimelineData?.config.span.end).toBe(DURATION_S);
  });

  it('nests each NVTX context beneath its owning worker resource', async () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    const catalog = {
      trace_start: 0,
      trace_end: 10,
      anomalies: {
        orphan_range_ends: '0',
        orphan_range_pops: '0',
        orphan_resource_destroys: '0',
        reused_range_ids: '0',
        reused_resource_handles: '0',
        total: '0',
        is_faithful: true,
      },
      domains: [
        {
          domain_id: '1',
          name: 'Runtime',
          color: '#2563eb',
          threads: [],
          categories: [],
          has_uncategorized: true,
        },
      ],
    } satisfies NvtxCatalog;
    const nvtxViewport = {
      viewport: { start: 0, end: 100 },
      domains: [
        {
          domain_id: '1',
          name: 'Runtime',
          color: '#2563eb',
          lanes: [
            {
              id: 'nvtx:1:marks',
              label: 'Marks',
              identity: { kind: 'marks' },
              ranges: [],
              marks: [],
            },
          ],
        },
      ],
      statistics: [],
    } satisfies NvtxViewportResponse;
    server.use(
      http.get('http://localhost:8000/api/engines/:engineId/contexts', ({ params }) =>
        HttpResponse.json({
          engine_id: params.engineId,
          context_resources: {
            'context-1': ['worker-1'],
            'context-other': ['worker-2'],
          },
        })
      ),
      http.get('http://localhost:8000/api/nvtx/contexts/context-1/catalog', () =>
        HttpResponse.text(
          JSON.stringify(catalog, (_key, value) =>
            typeof value === 'bigint' ? Number(value) : value
          ),
          { headers: { 'content-type': 'application/json' } }
        )
      ),
      http.post('http://localhost:8000/api/nvtx/contexts/context-1/viewport', () =>
        HttpResponse.text(
          JSON.stringify(nvtxViewport, (_key, value) =>
            typeof value === 'bigint' ? Number(value) : value
          ),
          { headers: { 'content-type': 'application/json' } }
        )
      )
    );

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle('worker-1')} />
      </JotaiProvider>
    );

    await waitFor(() => expect(findCapturedItem('nvtx:context-1:root')).toBeDefined());
    expect(capturedTreeData).toHaveLength(1);
    expect(findCapturedItem('worker-1')?.children?.map(item => item.id)).toContain(
      'nvtx:context-1:root'
    );
    expect(findCapturedItem('nvtx:context-other:root')).toBeUndefined();
    await waitFor(() =>
      expect(findCapturedItem('nvtx:context-1:root')?.children?.[0].id).toBe(
        'nvtx:context-1:domain:1'
      )
    );
  });

  it('omits the NVTX group when every context has no NVTX stream', async () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    let catalogRequests = 0;
    server.use(
      http.get('http://localhost:8000/api/engines/:engineId/contexts', ({ params }) =>
        HttpResponse.json({
          engine_id: params.engineId,
          context_resources: {
            'context-1': ['engine-1'],
            'context-2': ['engine-1'],
          },
        })
      ),
      http.get('http://localhost:8000/api/nvtx/contexts/:contextId/catalog', () => {
        catalogRequests += 1;
        return new HttpResponse(null, { status: 404 });
      })
    );

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await waitFor(() => expect(catalogRequests).toBe(2));
    await waitFor(() => expect(findCapturedItem('nvtx:context-1:root')).toBeUndefined());
    expect(findCapturedItem('nvtx:context-2:root')).toBeUndefined();
    expect(capturedTreeData[0].id).toBe(ROOT_GROUP_ID);
  });

  it('retains successful contexts when another NVTX catalog fails', async () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    server.use(
      http.get('http://localhost:8000/api/engines/:engineId/contexts', ({ params }) =>
        HttpResponse.json({
          engine_id: params.engineId,
          context_resources: {
            'context-good': ['engine-1'],
            'context-failed': ['engine-1'],
          },
        })
      ),
      http.get('http://localhost:8000/api/nvtx/contexts/context-good/catalog', () =>
        HttpResponse.json({
          trace_start: 0,
          trace_end: 10,
          anomalies: {
            orphan_range_ends: '0',
            orphan_range_pops: '0',
            orphan_resource_destroys: '0',
            reused_range_ids: '0',
            reused_resource_handles: '0',
            total: '0',
            is_faithful: true,
          },
          domains: [
            {
              domain_id: '1',
              name: 'Runtime',
              color: '#2563eb',
              threads: [],
              categories: [],
              has_uncategorized: true,
            },
          ],
        })
      ),
      http.get('http://localhost:8000/api/nvtx/contexts/context-failed/catalog', () =>
        HttpResponse.json({ message: 'internal' }, { status: 500 })
      ),
      http.post('http://localhost:8000/api/nvtx/contexts/context-good/viewport', () =>
        HttpResponse.json({
          viewport: { start: 0, end: 100 },
          domains: [
            {
              domain_id: '1',
              name: 'Runtime',
              color: '#2563eb',
              lanes: [
                {
                  id: 'nvtx:1:marks',
                  label: 'Marks',
                  identity: { kind: 'marks' },
                  ranges: [],
                  marks: [],
                },
              ],
            },
          ],
          statistics: [],
        })
      )
    );

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await waitFor(() => {
      expect(findCapturedItem('nvtx:context-good:domain:1')).toBeDefined();
      expect(findCapturedItem('nvtx:context-failed:catalog-error')).toBeDefined();
    });
    expect(capturedTreeData[0].id).toBe(ROOT_GROUP_ID);
  });

  it('shows the filtered-empty state only after an NVTX stream is available', async () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    server.use(
      http.get('http://localhost:8000/api/engines/:engineId/contexts', ({ params }) =>
        HttpResponse.json({
          engine_id: params.engineId,
          context_resources: { 'context-1': ['engine-1'] },
        })
      ),
      http.get('http://localhost:8000/api/nvtx/contexts/context-1/catalog', () =>
        HttpResponse.json({
          trace_start: 0,
          trace_end: 10,
          anomalies: {
            orphan_range_ends: '0',
            orphan_range_pops: '0',
            orphan_resource_destroys: '0',
            reused_range_ids: '0',
            reused_resource_handles: '0',
            total: '0',
            is_faithful: true,
          },
          domains: [
            {
              domain_id: '1',
              name: 'Runtime',
              color: '#2563eb',
              threads: [],
              categories: [],
              has_uncategorized: true,
            },
          ],
        })
      ),
      http.post('http://localhost:8000/api/nvtx/contexts/context-1/viewport', () =>
        HttpResponse.json({
          viewport: { start: 0, end: 100 },
          domains: [],
          statistics: [],
        })
      )
    );

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await waitFor(() =>
      expect(findCapturedItem('nvtx:context-1:root')?.children?.[0].id).toBe('nvtx:context-1:empty')
    );
  });
});
