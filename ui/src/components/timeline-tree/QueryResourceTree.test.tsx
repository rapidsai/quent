// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { waitFor, act, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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
  NVTX_LANE_ROW_TYPE,
  NVTX_SECTION_ROW_TYPE,
  OPERATOR_TIMELINE_ROW_TYPE,
  type TreeTableItem,
} from '@quent/components';
import { pinnedIdsAtom, resourceFilterAtom } from '@/atoms/resourceTree';
import { EMPTY_RESOURCE_FILTER } from '@/features/resource-filter/resourceFilter';
import type { ResourceTimelineSubRow } from './sub-rows';

// ---------------------------------------------------------------------------
// Mock heavy/visual dependencies so tests run without a real browser/canvas
// ---------------------------------------------------------------------------

vi.mock('@quent/hooks', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/hooks')>();
  return {
    ...actual,
    useBulkTimelines: (args: { extraItems?: TreeTableItem[] }) => {
      capturedBulkExtraItems = args.extraItems;
      return { handleZoomChange: vi.fn(), handleExpand: vi.fn() };
    },
    useHighlightedItemIds: () => new Set<string>(),
  };
});

vi.mock('./useFullDurationZeroUtilizationResourceIds', () => ({
  useFullDurationZeroUtilizationResourceIds: () => new Set<string>(),
}));

vi.mock('@/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'light', setTheme: vi.fn() }),
  THEME_DARK: 'dark',
  THEME_LIGHT: 'light',
}));

// Capture the timelineData prop passed to TimelineController on every render
let capturedTimelineData: SingleTimelineResponse | null | undefined = undefined;
let capturedTreeData: TreeTableItem[] = [];
let capturedHighlightedIds = new Set<string>();
let capturedExpandedIds = new Set<string>();
let capturedPinnedItems: TreeTableItem[] = [];
let capturedHiddenPinnedCount = 0;
let capturedPinnedCollapse: { isCollapsed: boolean; toggle?: () => void } = { isCollapsed: false };
let capturedReorderPinned: ((activeId: string, overId: string) => void) | undefined;
let capturedBulkExtraItems: TreeTableItem[] | undefined;
let capturedOnExpandChange: ((itemId: string, isExpanded: boolean) => void) | undefined;
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
        render?: (props: {
          item: TreeTableItem;
          level?: number;
          isPinned?: boolean;
        }) => React.ReactNode;
      }>;
      controlledExpandedIds?: Set<string>;
      data: TreeTableItem[];
      highlightedItemIds?: Set<string>;
      onExpandChange?: (itemId: string, isExpanded: boolean) => void;
      pinnedItems?: TreeTableItem[];
      hiddenPinnedCount?: number;
      isPinnedCollapsed?: boolean;
      onTogglePinnedCollapsed?: () => void;
      onReorderPinned?: (activeId: string, overId: string) => void;
    }) => {
      capturedTreeData = props.data;
      capturedPinnedItems = props.pinnedItems ?? [];
      capturedHiddenPinnedCount = props.hiddenPinnedCount ?? 0;
      capturedPinnedCollapse = {
        isCollapsed: props.isPinnedCollapsed ?? false,
        toggle: props.onTogglePinnedCollapsed,
      };
      capturedReorderPinned = props.onReorderPinned;
      capturedHighlightedIds = props.highlightedItemIds ?? new Set();
      capturedExpandedIds = props.controlledExpandedIds ?? new Set();
      capturedOnExpandChange = props.onExpandChange;
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
          {props.pinnedItems?.map(item => (
            <div key={`pinned-${item.id}`} data-testid="pinned-row">
              {props.columns[0]?.render?.({ item, level: 0, isPinned: true })}
            </div>
          ))}
        </>
      );
    },
    InlineSelector: (props: (typeof capturedInlineSelectors)[number]) => {
      capturedInlineSelectors.push(props);
      return <div data-testid={props.id} />;
    },
    ResourceColumn: (props: { description?: React.ReactNode }) => props.description ?? null,
    UsageColumn: () => null,
    TimelineToolbar: ({ filters }: { filters?: React.ReactNode }) => filters,
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
const ENGINE_ID = 'engine-1';
const NESTED_GROUP_ID = 'rg-1';
const RESOURCE_ID = 'res-1';
const RESOURCE_TYPE = 'GPU';

/** Minimal QueryBundle that causes the root timeline query to be enabled. */
const makeBundle = (workerId: string | null = null): QueryBundle<EntityRef> =>
  ({
    query_id: 'test-query',
    entities: {
      engine: { id: ENGINE_ID },
      query: { id: 'query-1' },
      workers: {},
      plans: {},
      operators: {},
      ports: {},
      resource_types: { [RESOURCE_TYPE]: { display_order: [{ Type: 'task' }], capacities: [] } },
      resource_group_types: {},
      resources: {
        [RESOURCE_ID]: { id: RESOURCE_ID, instance_name: 'GPU 0', type_name: RESOURCE_TYPE },
      },
      resource_groups: {},
      fsm_types: {},
    },
    resource_tree: {
      ResourceGroup: {
        id: { Engine: ENGINE_ID },
        children: [{ Resource: { Resource: RESOURCE_ID } }],
      },
    },
    plan_tree: { id: 'plan-1', worker: workerId, children: [] },
    unique_operator_names: [],
    quantity_specs: {},
    start_time_unix_ns: 0n,
    duration_s: DURATION_S,
  }) as unknown as QueryBundle<EntityRef>;

const makeNestedBundle = (): QueryBundle<EntityRef> => {
  const bundle = makeBundle();
  const nestedResourceGroup = {
    id: NESTED_GROUP_ID,
    instance_name: 'Worker 0',
    type_name: 'WorkerGroup',
    parent_group_id: ENGINE_ID,
  } satisfies QueryBundle<EntityRef>['entities']['resource_groups'][string];
  const nestedResourceTree = {
    ResourceGroup: {
      id: { Engine: ENGINE_ID },
      children: [
        {
          ResourceGroup: {
            id: { ResourceGroup: NESTED_GROUP_ID },
            children: [{ Resource: { Resource: RESOURCE_ID } }],
          },
        },
      ],
    },
  } satisfies QueryBundle<EntityRef>['resource_tree'];

  return {
    ...bundle,
    entities: {
      ...bundle.entities,
      resource_groups: {
        [NESTED_GROUP_ID]: nestedResourceGroup,
      },
    },
    resource_tree: nestedResourceTree,
  };
};

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
  label: 'Custom',
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
  capturedHighlightedIds = new Set();
  capturedExpandedIds = new Set();
  capturedPinnedItems = [];
  capturedHiddenPinnedCount = 0;
  capturedPinnedCollapse = { isCollapsed: false };
  capturedReorderPinned = undefined;
  capturedBulkExtraItems = undefined;
  capturedOnExpandChange = undefined;
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
          resourceId: ENGINE_ID,
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
      resourceId: ENGINE_ID,
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
          source_domain_ids: ['1', '2'],
          name: 'CCCL',
          color: '#76b900ff',
          threads: [],
          categories: [
            { category_id: 7, name: 'Compute' },
            { category_id: 8, name: '<category 8 has conflicting names: Decode | Encode>' },
          ],
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
      { value: '8', label: '<category 8 has conflicting names: Decode | Encode>' },
      { value: '__uncategorized__', label: 'Uncategorized' },
    ]);
    act(() => categorySelector?.onChange('nvtx-category-1', '7'));

    await waitFor(() => {
      const calls = vi.mocked(clientApi.useNvtxStream).mock.calls;
      const liveCall = [...calls].reverse().find(call => call[3]?.categoryFilters != null);
      expect(liveCall?.[3]?.categoryFilters?.get('1')).toEqual({
        categoryId: 7,
        includeUncategorized: false,
      });
    });

    const domainSelector = capturedInlineSelectors.find(selector => selector.id === 'nvtx-domain');
    expect(domainSelector?.options).toEqual([
      { value: '__all__', label: 'All' },
      { value: '1', label: 'CCCL' },
    ]);
    capturedInlineSelectors = [];
    act(() => domainSelector?.onChange('nvtx-domain', '1'));

    await waitFor(() =>
      expect(
        capturedInlineSelectors.find(selector => selector.id === 'nvtx-category-1')?.value
      ).toBe('7')
    );
  });

  it('qualifies duplicate domain names with their raw sources', () => {
    const catalog = {
      domains: [
        {
          domain_id: '0',
          source_domain_ids: ['0'],
          name: 'default domain',
          color: '#76b900ff',
          threads: [],
          categories: [],
          has_uncategorized: true,
        },
        {
          domain_id: '9',
          source_domain_ids: ['9'],
          name: 'default domain',
          color: '#2563ebff',
          threads: [],
          categories: [],
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

    const { getByText } = renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    expect(
      capturedInlineSelectors.find(selector => selector.id === 'nvtx-domain')?.options
    ).toEqual([
      { value: '__all__', label: 'All' },
      { value: '0', label: 'default domain · source 0' },
      { value: '9', label: 'default domain · source 9' },
    ]);
    expect(getByText('default domain · source 0')).toBeInTheDocument();
    expect(getByText('default domain · source 9')).toBeInTheDocument();
  });

  it('applies the shared text search to NVTX rows', async () => {
    const catalog = {
      domains: [
        {
          domain_id: '1',
          source_domain_ids: ['1'],
          name: 'Domain 1',
          color: '#76b900ff',
          threads: [{ thread_id: 7, name: 'worker 7' }],
          categories: [],
          has_uncategorized: false,
        },
      ],
    } as unknown as NvtxCatalog;
    const viewport = {
      viewport: { start: 0, end: DURATION_S },
      domains: [
        {
          domain_id: '1',
          source_domain_ids: ['1'],
          name: 'Domain 1',
          color: '#76b900ff',
          lanes: [
            {
              id: 'thread-7',
              label: 'worker 7',
              identity: { kind: 'thread', source_domain_id: '1', thread_id: 7, depth: 0 },
              ranges: [],
              marks: [],
            },
          ],
        },
      ],
      statistics: [],
    } as unknown as NonNullable<ReturnType<typeof clientApi.useNvtxStream>['viewport']>;
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.useNvtxStream).mockReturnValue({
      contextId: 'context-1',
      catalog,
      viewport,
      isLoading: false,
    });
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'worker 7',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: false,
    });

    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await waitFor(() => expect(collectRowTypes(capturedTreeData)).toContain(NVTX_LANE_ROW_TYPE));
    expect(capturedTreeData).toHaveLength(1);
    expect(collectRowTypes(capturedTreeData)).toEqual(
      expect.arrayContaining([NVTX_SECTION_ROW_TYPE, NVTX_LANE_ROW_TYPE])
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

describe('QueryResourceTree — resource filtering', () => {
  beforeEach(() => {
    capturedTreeData = [];
    capturedHighlightedIds = new Set();
    capturedExpandedIds = new Set();
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  it('shows matching resources without nonmatching ancestor rows', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'GPU 0',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: false,
    });

    const { getByRole } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await userEvent.click(getByRole('button', { name: 'Resource filters, 1 selected filter' }));
    expect(getByRole('textbox', { name: 'Search resource and NVTX tree labels' })).toHaveValue(
      'GPU 0'
    );
    await waitFor(() => expect(capturedTreeData[0]?.id).toBe(RESOURCE_ID));
    expect(capturedTreeData[0]?.id).toBe(RESOURCE_ID);
    expect(capturedTreeData[0]?.children).toEqual([]);
    expect(collectRowTypes(capturedTreeData)).toContain(LONG_ENTITIES_ROW_TYPE);
    expect(capturedHighlightedIds).not.toContain(RESOURCE_ID);
  });

  it('allows matching resource rows to be expanded while Show All is selected', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'GPU 0',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: true,
    });

    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle(RESOURCE_ID)} />
      </JotaiProvider>
    );

    await waitFor(() => expect(capturedTreeData[0]?.id).toBe(ENGINE_ID));
    expect(capturedExpandedIds).not.toContain(RESOURCE_ID);

    act(() => capturedOnExpandChange?.(RESOURCE_ID, true));
    await waitFor(() => expect(capturedExpandedIds).toContain(RESOURCE_ID));

    act(() => capturedOnExpandChange?.(RESOURCE_ID, false));
    await waitFor(() => expect(capturedExpandedIds).not.toContain(RESOURCE_ID));
  });

  it('shows the whole tree and highlights matches when Show All is selected', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'GPU 0',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: true,
    });

    const { getByRole } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await userEvent.click(getByRole('button', { name: 'Resource filters, 1 selected filter' }));
    expect(getByRole('checkbox', { name: 'Show All' })).toBeChecked();
    await waitFor(() => expect(capturedTreeData[0]?.id).toBe(ENGINE_ID));
    expect(capturedTreeData[0]?.children?.[0]?.id).toBe(RESOURCE_ID);
    expect(capturedHighlightedIds).toContain(RESOURCE_ID);
    expect(capturedExpandedIds).toContain(ENGINE_ID);
  });

  it('auto-expands every ancestor of found resources while Show All is selected', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'GPU 0',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: true,
    });

    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree
          engineId="engine-1"
          queryBundle={makeNestedBundle()}
          seedRootExpanded={false}
        />
      </JotaiProvider>
    );

    await waitFor(() => {
      expect(capturedExpandedIds).toContain(ENGINE_ID);
      expect(capturedExpandedIds).toContain(NESTED_GROUP_ID);
    });
  });

  it('auto-expands matching ancestors for a type-only filter with Show All selected', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: '',
      resourceTypes: [RESOURCE_TYPE],
      fsmTypes: [],
      showOthers: true,
    });

    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree
          engineId="engine-1"
          queryBundle={makeNestedBundle()}
          seedRootExpanded={false}
        />
      </JotaiProvider>
    );

    await waitFor(() => {
      expect(capturedExpandedIds).toContain(ENGINE_ID);
      expect(capturedExpandedIds).toContain(NESTED_GROUP_ID);
    });
  });

  it('shows an empty state when no resources match', async () => {
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'missing',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: false,
    });

    const { findByText } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    expect(await findByText('No resources match this filter.')).toBeInTheDocument();
    expect(capturedTreeData).toEqual([]);
  });

  it('shows an empty state with Show All selected when neither tree has matches', async () => {
    vi.mocked(clientApi.useNvtxStream).mockReturnValue({
      contextId: 'context-1',
      catalog: {
        domains: [
          {
            domain_id: '1',
            source_domain_ids: ['1'],
            name: 'Domain 1',
            color: '#76b900ff',
            threads: [{ thread_id: 7, name: 'worker 7' }],
            categories: [],
            has_uncategorized: false,
          },
        ],
      } as unknown as NvtxCatalog,
      viewport: null,
      isLoading: false,
    });
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'missing',
      resourceTypes: [],
      fsmTypes: [],
      showOthers: true,
    });

    const { findByText } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    expect(await findByText('No resources match this filter.')).toBeInTheDocument();
    expect(capturedTreeData).toEqual([]);
  });
});

const SECOND_RESOURCE_ID = 'res-2';

/** Two GPUs side by side, so a filter can keep one and remove the other. */
const makeTwoResourceBundle = (): QueryBundle<EntityRef> => {
  const bundle = makeBundle();
  return {
    ...bundle,
    entities: {
      ...bundle.entities,
      resources: {
        ...bundle.entities.resources,
        [SECOND_RESOURCE_ID]: {
          id: SECOND_RESOURCE_ID,
          instance_name: 'GPU 1',
          type_name: RESOURCE_TYPE,
        },
      },
    },
    resource_tree: {
      ResourceGroup: {
        id: { Engine: ENGINE_ID },
        children: [
          { Resource: { Resource: RESOURCE_ID } },
          { Resource: { Resource: SECOND_RESOURCE_ID } },
        ],
      },
    },
  } as unknown as QueryBundle<EntityRef>;
};

/** Engine > worker-0 > GPU 0, with an engine name that must not show up on pinned rows. */
const makeWorkerBundle = (): QueryBundle<EntityRef> => {
  const bundle = makeBundle();
  const workerId = 'worker-id-1';
  return {
    ...bundle,
    entities: {
      ...bundle.entities,
      engine: { ...bundle.entities.engine, instance_name: 'holodeck-1234' },
      workers: {
        [workerId]: { id: workerId, parent_engine_id: ENGINE_ID, instance_name: 'worker-0' },
      },
    },
    resource_tree: {
      ResourceGroup: {
        id: { Engine: ENGINE_ID },
        children: [
          {
            ResourceGroup: {
              id: { Worker: workerId },
              children: [{ Resource: { Resource: RESOURCE_ID } }],
            },
          },
        ],
      },
    },
  } as unknown as QueryBundle<EntityRef>;
};

const pinnedIds = () => capturedPinnedItems.map(item => item.id);

describe('QueryResourceTree — pinned rows', () => {
  beforeEach(() => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  function renderWithPins(ids: string[], bundle = makeTwoResourceBundle()) {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set(ids));
    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={bundle} />
      </JotaiProvider>
    );
    return store;
  }

  it('shows a pinned resource in the pinned section', async () => {
    renderWithPins([SECOND_RESOURCE_ID]);

    await waitFor(() => expect(pinnedIds()).toEqual([SECOND_RESOURCE_ID]));
    expect(capturedHiddenPinnedCount).toBe(0);
  });

  it('shows a pinned Entities lane', async () => {
    const laneId = `__long_entities__${RESOURCE_ID}`;
    renderWithPins([laneId]);

    await waitFor(() => expect(pinnedIds()).toEqual([laneId]));
  });

  it('shows nothing and counts nothing for pinned ids that match no row', async () => {
    renderWithPins(['not-a-row']);

    await waitFor(() => expect(capturedTreeData.length).toBeGreaterThan(0));
    expect(pinnedIds()).toEqual([]);
    expect(capturedHiddenPinnedCount).toBe(0);
  });

  it('hides a pin when a filter removes its row, and restores it when the filter clears', async () => {
    const laneId = `__long_entities__${SECOND_RESOURCE_ID}`;
    const store = renderWithPins([SECOND_RESOURCE_ID, laneId, RESOURCE_ID]);
    await waitFor(() => expect(pinnedIds()).toHaveLength(3));

    act(() =>
      store.set(resourceFilterAtom, {
        search: 'GPU 0',
        resourceTypes: [],
        fsmTypes: [],
        showOthers: false,
      })
    );

    await waitFor(() => expect(pinnedIds()).toEqual([RESOURCE_ID]));
    expect(capturedHiddenPinnedCount).toBe(2);

    act(() => store.set(resourceFilterAtom, EMPTY_RESOURCE_FILTER));

    await waitFor(() => expect(pinnedIds()).toHaveLength(3));
    expect(capturedHiddenPinnedCount).toBe(0);
    expect([...store.get(pinnedIdsAtom)]).toHaveLength(3);
  });

  it('keeps every pin while a filter only highlights matches', async () => {
    const store = renderWithPins([SECOND_RESOURCE_ID, RESOURCE_ID]);
    await waitFor(() => expect(pinnedIds()).toHaveLength(2));

    act(() =>
      store.set(resourceFilterAtom, {
        search: 'GPU 0',
        resourceTypes: [],
        fsmTypes: [],
        showOthers: true,
      })
    );

    await waitFor(() => expect(capturedHighlightedIds).toContain(RESOURCE_ID));
    expect(pinnedIds()).toHaveLength(2);
    expect(capturedHiddenPinnedCount).toBe(0);
  });

  it('collapses and expands the pinned section from its header', async () => {
    renderWithPins([RESOURCE_ID]);
    await waitFor(() => expect(capturedPinnedCollapse.toggle).toBeDefined());
    expect(capturedPinnedCollapse.isCollapsed).toBe(false);

    act(() => capturedPinnedCollapse.toggle?.());
    await waitFor(() => expect(capturedPinnedCollapse.isCollapsed).toBe(true));

    act(() => capturedPinnedCollapse.toggle?.());
    await waitFor(() => expect(capturedPinnedCollapse.isCollapsed).toBe(false));
  });

  it('asks for bulk timeline data for pinned resources but not for pinned sub-rows', async () => {
    renderWithPins([SECOND_RESOURCE_ID, `__long_entities__${RESOURCE_ID}`]);

    await waitFor(() =>
      expect(capturedBulkExtraItems?.map(item => item.id)).toEqual([SECOND_RESOURCE_ID])
    );
  });
});

describe('QueryResourceTree — pin buttons', () => {
  beforeEach(() => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  const pinButtonNames = (container: HTMLElement) =>
    within(container)
      .queryAllByRole('button', { name: /^(Pin|Unpin) / })
      .map(button => button.getAttribute('aria-label'));

  it('offers pinning on resource and Entities rows, not on the root group or NVTX rows', () => {
    vi.mocked(clientApi.useNvtxStream).mockReturnValue({
      contextId: 'context-1',
      catalog: {
        domains: [
          {
            domain_id: '1',
            source_domain_ids: ['1'],
            name: 'CCCL',
            color: '#76b900ff',
            threads: [],
            categories: [],
            has_uncategorized: false,
          },
        ],
      } as unknown as NvtxCatalog,
      viewport: null,
      isLoading: false,
    });

    const { container } = renderWithQuery(
      <JotaiProvider store={createStore()}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    expect(pinButtonNames(container)).toEqual(['Pin GPU 0', 'Pin Entities']);
  });

  it('pins a row when its button is clicked and unpins it from the pinned section', async () => {
    const store = createStore();
    const { getByRole, getAllByRole, getByTestId, queryAllByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    await userEvent.click(getByRole('button', { name: 'Pin GPU 0' }));

    expect([...store.get(pinnedIdsAtom)]).toEqual([RESOURCE_ID]);
    // One button on the row in the tree and one on its copy in the pinned section.
    const unpinButtons = getAllByRole('button', { name: 'Unpin GPU 0' });
    expect(unpinButtons).toHaveLength(2);
    unpinButtons.forEach(button => expect(button).toHaveAttribute('aria-pressed', 'true'));

    await userEvent.click(within(getByTestId('pinned-row')).getByRole('button'));

    expect(store.get(pinnedIdsAtom).size).toBe(0);
    expect(queryAllByTestId('pinned-row')).toHaveLength(0);
    expect(getByRole('button', { name: 'Pin GPU 0' })).toBeInTheDocument();
  });

  it('shows the worker under a pinned resource instead of the engine name', async () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set([RESOURCE_ID]));
    const { findAllByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeWorkerBundle()} />
      </JotaiProvider>
    );

    const [pinnedRow] = await findAllByTestId('pinned-row');

    expect(within(pinnedRow!).getByText('Worker:')).toBeInTheDocument();
    expect(within(pinnedRow!).getByText('worker-0')).toBeInTheDocument();
    expect(within(pinnedRow!).queryByText(/holodeck/)).not.toBeInTheDocument();
  });

  it('shows the worker on a pinned Entities row', async () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set([`__long_entities__${RESOURCE_ID}`]));
    const { findAllByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeWorkerBundle()} />
      </JotaiProvider>
    );

    const [pinnedRow] = await findAllByTestId('pinned-row');

    expect(within(pinnedRow!).getByText('Entities')).toBeInTheDocument();
    expect(within(pinnedRow!).getByText('worker-0')).toBeInTheDocument();
    expect(within(pinnedRow!).getByRole('button', { name: 'Unpin Entities' })).toBeInTheDocument();
  });

  it('does not label a pinned worker as belonging to itself', async () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set(['worker-id-1']));
    const { findAllByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeWorkerBundle()} />
      </JotaiProvider>
    );

    const [pinnedRow] = await findAllByTestId('pinned-row');

    expect(within(pinnedRow!).queryByText('Worker:')).not.toBeInTheDocument();
  });

  it('shows no worker line for a pinned row that has no worker', async () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set([RESOURCE_ID]));
    const { findAllByTestId } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );

    const [pinnedRow] = await findAllByTestId('pinned-row');

    expect(within(pinnedRow!).queryByText('Worker:')).not.toBeInTheDocument();
  });

  it('does not show the worker line on rows in the tree', async () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set([RESOURCE_ID]));
    const { findAllByTestId, queryAllByText } = renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeWorkerBundle()} />
      </JotaiProvider>
    );

    await findAllByTestId('pinned-row');

    // The one pinned row shows it; the tree rows do not.
    expect(queryAllByText('Worker:')).toHaveLength(1);
  });
});

describe('QueryResourceTree — arranging pinned rows', () => {
  const LANE_ID = `__long_entities__${RESOURCE_ID}`;

  beforeEach(() => {
    vi.mocked(clientApi.fetchSingleTimeline).mockResolvedValue(makeTimeline(0, DURATION_S));
    vi.mocked(clientApi.fetchBulkTimelines).mockResolvedValue({ entries: {} } as never);
  });

  function renderWithPins(ids: string[]) {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set(ids));
    renderWithQuery(
      <JotaiProvider store={store}>
        <QueryResourceTree engineId="engine-1" queryBundle={makeBundle()} />
      </JotaiProvider>
    );
    return store;
  }

  const pinnedOrder = () => capturedPinnedItems.map(item => item.id);

  it('lists pinned rows in the order they were pinned, not tree order', async () => {
    renderWithPins([LANE_ID, RESOURCE_ID]);

    await waitFor(() => expect(pinnedOrder()).toEqual([LANE_ID, RESOURCE_ID]));
  });

  it('moves a pinned row when it is dropped on another', async () => {
    const store = renderWithPins([RESOURCE_ID, LANE_ID]);
    await waitFor(() => expect(capturedReorderPinned).toBeDefined());

    act(() => capturedReorderPinned?.(RESOURCE_ID, LANE_ID));

    await waitFor(() => expect(pinnedOrder()).toEqual([LANE_ID, RESOURCE_ID]));
    expect([...store.get(pinnedIdsAtom)]).toEqual([LANE_ID, RESOURCE_ID]);
  });
});
