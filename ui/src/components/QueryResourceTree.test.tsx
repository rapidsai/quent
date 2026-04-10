// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { waitFor, act } from '@testing-library/react';
import { renderWithQuery } from '@/test/test-utils';
import { Provider as JotaiProvider, createStore } from 'jotai';
import { QueryResourceTree } from './QueryResourceTree';
import { applyBulkTimelineResponse } from '@/hooks/useBulkTimelineFetch';
import { timelineCacheKey, timelineDataAtom } from '@/atoms/timeline';
import type { SingleTimelineResponse } from '~quent/types/SingleTimelineResponse';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';

// ---------------------------------------------------------------------------
// Mock heavy/visual dependencies so tests run without a real browser/canvas
// ---------------------------------------------------------------------------

vi.mock('@/hooks/useBulkTimelines', () => ({
  useBulkTimelines: () => ({ handleZoomChange: vi.fn(), handleExpand: vi.fn() }),
}));

vi.mock('@/hooks/useExpandedIds', () => ({
  useExpandedIds: () => ({ expandedIds: new Set<string>(), handleExpandChange: vi.fn() }),
}));

vi.mock('@/hooks/useHighlightedItemIds', () => ({
  useHighlightedItemIds: () => new Set<string>(),
}));

// Capture the timelineData prop passed to TimelineController on every render
let capturedTimelineData: SingleTimelineResponse | null | undefined = undefined;
vi.mock('./timeline/TimelineController', () => ({
  TimelineController: (props: { timelineData?: SingleTimelineResponse | null }) => {
    capturedTimelineData = props.timelineData;
    return null;
  },
}));

// Render subHeaderContent so TimelineController is actually mounted
vi.mock('@/components/ui/tree-table', () => ({
  TreeTable: ({ columns }: { columns: Array<{ subHeaderContent?: React.ReactNode }> }) => {
    const col = columns.find(c => c.subHeaderContent != null);
    return <>{col?.subHeaderContent}</>;
  },
}));

vi.mock('./resource-tree/ResourceColumn', () => ({ ResourceColumn: () => null }));
vi.mock('./resource-tree/UsageColumn', () => ({ UsageColumn: () => null }));
vi.mock('./timeline/TimelineToolbar', () => ({ TimelineToolbar: () => null }));

import * as api from '@/services/api';
vi.mock('@/services/api', async importOriginal => {
  const actual = await importOriginal<typeof api>();
  return { ...actual, fetchSingleTimeline: vi.fn(), fetchBulkTimelines: vi.fn() };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DURATION_S = 100;
const ROOT_GROUP_ID = 'qg-1';
const RESOURCE_ID = 'res-1';
const RESOURCE_TYPE = 'GPU';

/** Minimal QueryBundle that causes the root timeline query to be enabled. */
const makeBundle = (): QueryBundle<EntityRef> =>
  ({
    query_id: 'test-query',
    entities: {
      engine: { id: 'engine-1' },
      query_group: { id: ROOT_GROUP_ID },
      query: { id: 'query-1' },
      workers: {},
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
        children: [{ Resource: { Resource: RESOURCE_ID } }],
      },
    },
    plan_tree: { id: 'plan-1', worker: null, children: [] },
    unique_operator_names: [],
    quantity_specs: {},
    start_time_unix_ns: 0n,
    duration_s: DURATION_S,
  }) as unknown as QueryBundle<EntityRef>;

const makeTimeline = (start: number, end: number): SingleTimelineResponse =>
  ({
    config: { span: { start, end }, bin_duration: 1, num_bins: BigInt(end - start) },
    data: { Binned: { series: {} } },
  }) as unknown as SingleTimelineResponse;

describe('QueryResourceTree — TimelineController always shows full-range data', () => {
  beforeEach(() => {
    capturedTimelineData = undefined;
    vi.mocked(api.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  it('passes full-range timeline data to TimelineController', async () => {
    const fullRange = makeTimeline(0, DURATION_S);
    vi.mocked(api.fetchSingleTimeline).mockResolvedValue(fullRange);

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
    vi.mocked(api.fetchSingleTimeline).mockResolvedValue(fullRange);

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
    expect(store.get(timelineDataAtom(cacheKey))?.config.span.start).toBe(25);

    // TimelineController must still show the full-range data — not the atom value.
    expect(capturedTimelineData?.config.span.start).toBe(0);
    expect(capturedTimelineData?.config.span.end).toBe(DURATION_S);
  });
});
