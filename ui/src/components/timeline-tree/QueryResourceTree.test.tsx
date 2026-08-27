// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { waitFor, act } from '@testing-library/react';
import { renderWithQuery } from '@/test/test-utils';
import { Provider as JotaiProvider, createStore } from 'jotai';
import { QueryResourceTree } from './QueryResourceTree';
import { applyBulkTimelineResponse, timelineCacheKey, useZoomRange } from '@quent/hooks';
import { timelineDataMapAtom } from '@quent/hooks/testing';
import type {
  SingleTimelineResponse,
  QueryBundle,
  EntityRef,
  FiniteStateMachine,
  NvtxCatalog,
} from '@quent/utils';
import {
  LONG_ENTITIES_ROW_TYPE,
  OPERATOR_TIMELINE_ROW_TYPE,
  type TreeTableItem,
} from '@quent/components';
import type { ResourceTimelineSubRow } from './sub-rows';

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
let capturedTreeData: TreeTableItem[] = [];
let capturedInlineSelectors: Array<{
  id: string;
  value: string;
  options: Array<string | { value: string; label: string }>;
  onChange: (id: string, value: string) => void;
}> = [];
let capturedLongEntityProps:
  | {
      onEntitySelect?: (fsm: FiniteStateMachine) => void;
      selectedEntityId?: string;
    }
  | undefined;

// Mock @quent/components: keep all actual exports but override heavy/visual ones
vi.mock('@quent/components', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/components')>();
  return {
    ...actual,
    TimelineController: (props: { timelineData?: SingleTimelineResponse | null }) => {
      capturedTimelineData = props.timelineData;
      return null;
    },
    TreeTable: (props: {
      columns: Array<{
        headerContent?: React.ReactNode;
        subHeaderContent?: React.ReactNode;
        render?: (props: { item: TreeTableItem; level?: number }) => React.ReactNode;
      }>;
      data: TreeTableItem[];
    }) => {
      capturedTreeData = props.data;
      const longEntityElement = props.columns[1]?.render?.({
        item: {
          id: actual.longEntitiesRowId(RESOURCE_ID),
          type: actual.LONG_ENTITIES_ROW_TYPE,
          entity: {} as TreeTableItem['entity'],
        },
      });
      if (React.isValidElement(longEntityElement)) {
        capturedLongEntityProps = longEntityElement.props as typeof capturedLongEntityProps;
      }

      const renderItems = (items: TreeTableItem[], level = 0): React.ReactNode =>
        items.map(item => (
          <React.Fragment key={item.id}>
            {props.columns[0]?.render?.({ item, level })}
            {item.children && renderItems(item.children, level + 1)}
          </React.Fragment>
        ));
      return (
        <>
          {props.columns.map((column, index) => (
            <React.Fragment key={index}>
              {column.headerContent}
              {column.subHeaderContent}
            </React.Fragment>
          ))}
          {renderItems(props.data)}
        </>
      );
    },
    InlineSelector: (props: (typeof capturedInlineSelectors)[number]) => {
      capturedInlineSelectors.push(props);
      return <div data-testid={props.id} />;
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
    useNvtxStream: vi.fn(),
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
const makeBundle = (workerId: string | null = null): QueryBundle<EntityRef> =>
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
    plan_tree: { id: 'plan-1', worker: workerId, children: [] },
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

function ViewportProbe() {
  const range = useZoomRange();
  return <output data-testid="viewport">{JSON.stringify(range)}</output>;
}

function collectRowTypes(items: TreeTableItem[]): string[] {
  return items.flatMap(item => [item.type, ...collectRowTypes(item.children ?? [])]);
}

const CUSTOM_SUB_ROW_TYPE = 'custom-sub-row';
const customSubRow: ResourceTimelineSubRow = {
  id: 'custom',
  injectRows: rootItem => ({
    ...rootItem,
    children: [
      ...(rootItem.children ?? []),
      {
        id: 'custom-sub-row',
        type: CUSTOM_SUB_ROW_TYPE,
        entity: {} as TreeTableItem['entity'],
      },
    ],
  }),
  matches: item => item.type === CUSTOM_SUB_ROW_TYPE,
  renderLabel: () => null,
  renderTimeline: () => null,
};

beforeEach(() => {
  capturedInlineSelectors = [];
  capturedTimelineData = undefined;
  capturedTreeData = [];
  capturedLongEntityProps = undefined;
  vi.mocked(clientApi.useNvtxStream).mockReturnValue({
    contextId: undefined,
    catalog: null,
    viewport: null,
    isLoading: false,
  });
});

describe('QueryResourceTree — TimelineController always shows full-range data', () => {
  beforeEach(() => {
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  it('deselects an entity when it is selected again', () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    const fsm = {
      id: 'entity-1',
      type_name: 'Task',
      instance_name: 'Task 1',
      transitions: [],
    } as FiniteStateMachine;

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    act(() => capturedLongEntityProps?.onEntitySelect?.(fsm));
    expect(capturedLongEntityProps?.selectedEntityId).toBe(fsm.id);

    act(() => capturedLongEntityProps?.onEntitySelect?.(fsm));
    expect(capturedLongEntityProps?.selectedEntityId).toBeUndefined();
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

  it('hydrates an imported initial viewport instead of the full query range', async () => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));

    const store = createStore();
    const { getByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree
          engineId="engine-1"
          queryBundle={makeBundle()}
          initialZoomRange={{ start: 25, end: 75 }}
        />
        <ViewportProbe />
      </JotaiProvider>
    );

    await waitFor(() =>
      expect(getByTestId('viewport')).toHaveTextContent(JSON.stringify({ start: 25, end: 75 }))
    );
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
          operatorIds: [],
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
});

describe('QueryResourceTree — NVTX filters', () => {
  it('renders category selectors on domain rows and keeps the selected domain header', async () => {
    const catalog = {
      domains: [
        {
          domain_id: '1',
          name: 'Domain 1',
          color: '#76b900ff',
          threads: [],
          categories: [{ category_id: 7, name: 'Compute' }],
          has_uncategorized: true,
        },
      ],
    } as unknown as NvtxCatalog;
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.useNvtxStream).mockReturnValue({
      contextId: 'context-1',
      catalog,
      viewport: null,
      isLoading: false,
    });

    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    const categorySelector = capturedInlineSelectors.find(
      selector => selector.id === 'nvtx-category-1'
    );
    expect(categorySelector?.options).toEqual([
      { value: '__all__', label: 'All' },
      { value: '7', label: 'Compute' },
      { value: '__uncategorized__', label: 'Uncategorized' },
    ]);
    act(() => categorySelector?.onChange('nvtx-category-1', '7'));

    await waitFor(() => {
      const calls = vi.mocked(clientApi.useNvtxStream).mock.calls;
      expect(calls[calls.length - 1]?.[3]?.categoryFilters?.get('1')).toEqual({
        categoryId: 7,
        includeUncategorized: false,
      });
    });

    const domainSelector = capturedInlineSelectors.find(selector => selector.id === 'nvtx-domain');
    capturedInlineSelectors = [];
    act(() => domainSelector?.onChange('nvtx-domain', '1'));

    await waitFor(() =>
      expect(
        capturedInlineSelectors.find(selector => selector.id === 'nvtx-category-1')?.value
      ).toBe('7')
    );
  });
});

describe('QueryResourceTree — configurable resource subrows', () => {
  beforeEach(() => {
    capturedTreeData = [];
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  it('renders the default subrow descriptors', () => {
    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle(RESOURCE_ID)} />
      </JotaiProvider>
    );

    const rowTypes = collectRowTypes(capturedTreeData);
    expect(rowTypes).toContain(OPERATOR_TIMELINE_ROW_TYPE);
    expect(rowTypes).toContain(LONG_ENTITIES_ROW_TYPE);
  });

  it('can render without any subrows', () => {
    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree
          engineId="engine-1"
          queryBundle={makeBundle(RESOURCE_ID)}
          resourceSubRows={[]}
        />
      </JotaiProvider>
    );

    const rowTypes = collectRowTypes(capturedTreeData);
    expect(rowTypes).not.toContain(OPERATOR_TIMELINE_ROW_TYPE);
    expect(rowTypes).not.toContain(LONG_ENTITIES_ROW_TYPE);
  });

  it('renders arbitrary supplied subrow descriptors', () => {
    renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree
          engineId="engine-1"
          queryBundle={makeBundle(RESOURCE_ID)}
          resourceSubRows={[customSubRow]}
        />
      </JotaiProvider>
    );

    const rowTypes = collectRowTypes(capturedTreeData);
    expect(rowTypes).toContain(CUSTOM_SUB_ROW_TYPE);
    expect(rowTypes).not.toContain(OPERATOR_TIMELINE_ROW_TYPE);
    expect(rowTypes).not.toContain(LONG_ENTITIES_ROW_TYPE);
  });
});
