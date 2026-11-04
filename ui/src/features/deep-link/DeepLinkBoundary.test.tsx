// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect } from 'react';
import { gzipSync, strToU8 } from 'fflate';
import { Provider as JotaiProvider, createStore, useAtomValue, useSetAtom } from 'jotai';
import {
  useDebouncedZoomRange,
  useHydrateTimelineAtoms,
  useOperatorSelection,
  useSerializableViewState,
  useSetDebouncedZoomRange,
  useSetZoomRange,
  useZoomRange,
} from '@quent/hooks';
import { NVTX_SECTION_ID, toast, Toaster } from '@quent/components';
import type { Operator } from '@quent/utils';
import { render, screen, waitFor, userEvent } from '@/test/test-utils';
import { entitiesTableStateAtom, type EntitiesTableState } from '@/atoms/entitiesTable';
import {
  expandedIdsAtom,
  pinnedIdsAtom,
  resourceFilterAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import {
  EMPTY_RESOURCE_FILTER,
  type ResourceFilter,
} from '@/features/resource-filter/resourceFilter';
import {
  OPERATOR_TABLE_INDEX_ORDER,
  OPERATOR_TABLE_PERSIST_KEY,
} from '@/components/operator-table/types';
import { CopyLinkButton } from './CopyLinkButton';
import { DeepLinkBoundary } from './DeepLinkBoundary';
import { decodeDeepLinkState, encodeDeepLinkState } from './deepLink.codec';
import { useDeepLink } from './deepLink.context';
import { MAX_EXPANDED_RESOURCE_IDS, MAX_SELECTED_NODE_IDS } from './deepLink.schema';

const BOUNDARY_PROPS = {
  engineId: 'e',
  queryId: 'q',
  activeTab: 'timeline' as const,
  initialPlanId: '',
  durationSeconds: 100,
  isQueryReady: true,
  operators: [],
};

const RESOURCE_A_ID = '01a025ff-ea8b-7881-9d31-72a275872c9d';
const RESOURCE_B_ID = '01a025ff-ea8b-7881-9d31-72a275872c9e';

function encodeRawState(value: unknown): string {
  const compressed = gzipSync(strToU8(JSON.stringify(value)), { level: 9, mtime: 0 });
  let binary = '';
  for (const byte of compressed) {
    binary += String.fromCharCode(byte);
  }
  return `v3.${btoa(binary).replace(/\+/gu, '-').replace(/\//gu, '_').replace(/=+$/u, '')}`;
}

const DEEP_LINK_OPERATORS: Operator[] = [
  {
    id: 'operator-a',
    plan_id: null,
    parent_operator_ids: [],
    instance_name: 'Operator A',
    operator_type_name: null,
    custom_attributes: [],
    statistics: null,
    active_span: null,
  },
  {
    id: 'operator-child',
    plan_id: null,
    parent_operator_ids: ['operator-a'],
    instance_name: 'Operator Child',
    operator_type_name: null,
    custom_attributes: [],
    statistics: null,
    active_span: null,
  },
];

function ViewportProbe() {
  const immediate = useZoomRange();
  const debounced = useDebouncedZoomRange();
  return <output data-testid="viewport">{JSON.stringify({ immediate, debounced })}</output>;
}

function IntakeStatusProbe() {
  const deepLink = useDeepLink();
  return <output data-testid="intake-status">{deepLink?.intakeStatus.kind}</output>;
}

function InitialEntitiesProbe() {
  const state = useAtomValue(entitiesTableStateAtom);
  return <output data-testid="initial-entities">{JSON.stringify(state)}</output>;
}

function ExpandedRowsProbe() {
  const expandedIds = useAtomValue(expandedIdsAtom);
  return <output data-testid="expanded-rows">{JSON.stringify([...expandedIds].sort())}</output>;
}

function SerializableStateProbe() {
  const { read } = useSerializableViewState({
    operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
    operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
    operators: [],
  });
  const expandedIds = useAtomValue(expandedIdsAtom);
  const selectedTypes = useAtomValue(selectedTypesAtom);
  const selectedFsmTypes = useAtomValue(selectedFsmTypesAtom);
  const rootResourceType = useAtomValue(rootResourceTypeAtom);
  const resourceFilter = useAtomValue(resourceFilterAtom);
  return (
    <output data-testid="serializable-state">
      {JSON.stringify({
        view: read(),
        resources: {
          expandedIds: [...expandedIds].sort(),
          selectedTypes: [...selectedTypes],
          selectedFsmTypes: [...selectedFsmTypes],
          rootResourceType,
          resourceFilter,
        },
      })}
    </output>
  );
}

function OperatorSelectionProbe() {
  const selection = useOperatorSelection();
  return (
    <output data-testid="operator-selection">
      {JSON.stringify(
        [...selection.selections].map(([id, value]) => ({
          id,
          label: value.label,
          operatorIds: [...value.operatorIds],
        }))
      )}
    </output>
  );
}

function SeedViewport({ start, end }: { start: number; end: number }) {
  const setImmediate = useSetZoomRange();
  const setDebounced = useSetDebouncedZoomRange();

  useLayoutEffect(() => {
    setImmediate({ start, end });
    setDebounced({ start, end });
  }, [end, setDebounced, setImmediate, start]);
  return null;
}

function SeedExpandedRows({ ids }: { ids: string[] }) {
  const setExpandedIds = useSetAtom(expandedIdsAtom);

  useLayoutEffect(() => {
    setExpandedIds(new Set(ids));
  }, [ids, setExpandedIds]);
  return null;
}

function SeedSelectedOperators({ ids }: { ids: string[] }) {
  const { hydrate } = useSerializableViewState({
    operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
    operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
    operators: [],
  });

  useLayoutEffect(() => {
    hydrate({ selection: { operatorNodeIds: ids } });
  }, [hydrate, ids]);
  return null;
}

function SeedPinnedRows({ ids }: { ids: string[] }) {
  const setPinnedIds = useSetAtom(pinnedIdsAtom);

  useLayoutEffect(() => {
    setPinnedIds(new Set(ids));
  }, [ids, setPinnedIds]);
  return null;
}

function PinnedRowsProbe() {
  const pinnedIds = useAtomValue(pinnedIdsAtom);
  return <output data-testid="pinned-rows">{JSON.stringify([...pinnedIds])}</output>;
}

function SeedEmptyDataFlowDimensions() {
  const { hydrate } = useSerializableViewState({
    operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
    operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
    operators: [],
  });

  useLayoutEffect(() => {
    hydrate({ dataFlow: { dimensions: [] } });
  }, [hydrate]);
  return null;
}

function SeedResourceFilter({ filter }: { filter: ResourceFilter }) {
  const setResourceFilter = useSetAtom(resourceFilterAtom);

  useLayoutEffect(() => {
    setResourceFilter(filter);
  }, [filter, setResourceFilter]);
  return null;
}

function SeedEntitiesState({ state }: { state: EntitiesTableState }) {
  const setEntitiesState = useSetAtom(entitiesTableStateAtom);

  useLayoutEffect(() => {
    setEntitiesState(state);
  }, [setEntitiesState, state]);
  return null;
}

function HydrateTimelineDuringRender() {
  useHydrateTimelineAtoms({
    zoomRange: { start: 0, end: 100 },
    debouncedZoomRange: { start: 0, end: 100 },
    startTimeMs: 0,
  });
  return null;
}

describe('DeepLinkBoundary', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates timeline viewport and expanded rows before rendering children', () => {
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
      resources: { expandedRowIds: [RESOURCE_B_ID, RESOURCE_A_ID] },
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded.value}>
          <ViewportProbe />
          <ExpandedRowsProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('viewport')).toHaveTextContent(
      JSON.stringify({
        immediate: { start: 10, end: 40 },
        debounced: { start: 10, end: 40 },
      })
    );
    expect(screen.getByTestId('expanded-rows')).toHaveTextContent(
      JSON.stringify([RESOURCE_A_ID, RESOURCE_B_ID])
    );
  });

  it('removes consumed shared state from the address bar', () => {
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }
    window.history.replaceState(
      null,
      '',
      `/profile/engine/e/query/q/timeline?s=${encodeURIComponent(encoded.value)}&unrelated=kept#view`
    );

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded.value}>
          <ViewportProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(window.location.search).toBe('?unrelated=kept');
    expect(window.location.hash).toBe('#view');
  });

  it('hydrates a legacy v1 viewport without version-specific branching', () => {
    render(
      <JotaiProvider>
        <DeepLinkBoundary
          {...BOUNDARY_PROPS}
          encodedState="v1.H4sIAAAAAAACA6tWqsrPzw1KzEtPVbKqViouSSwqUbIy0FFKzUsB0nomBua1tQAidcVYJQAAAA"
        >
          <ViewportProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('viewport')).toHaveTextContent(
      JSON.stringify({
        immediate: { start: 0, end: 0.407 },
        debounced: { start: 0, end: 0.407 },
      })
    );
  });

  it('hydrates comprehensive view state before rendering children', () => {
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
      selection: {
        planId: 'plan-a',
        operatorNodeIds: ['operator-a', 'operator-child', 'operator-unknown'],
      },
      resources: {
        expandedRowIds: ['worker-a'],
        resourceFilter: {
          search: 'worker',
          resourceTypes: ['channel', 'memory'],
          fsmTypes: ['task', 'transfer'],
          showOthers: true,
        },
        rootResourceType: 'channel',
        resourceTypeSelections: [{ rowId: 'worker-a', resourceType: 'memory' }],
        fsmSelections: [{ rowId: 'worker-a', fsmType: 'task' }],
      },
      dag: {
        nodeColorField: 'duration_s',
        nodeColorPalette: 'viridis',
        edgeWidthField: 'bytes',
        edgeColorField: 'rows',
        edgeColorPalette: 'purple',
        nodeLabelField: 'type',
        layoutDirection: 'top-to-bottom',
        scaleType: 'linear',
      },
      dataFlow: {
        enabled: false,
        measure: 'bytes',
        labelMeasure: 'tasks',
        dimensions: ['filesystem'],
        playheadS: 25,
      },
      operatorTable: {
        groupingOrder: ['partition', 'item_type', 'item'],
        enabledGroups: ['partition', 'item_type'],
        visibleStats: ['duration_s', 'spill_bytes'],
        aggregation: 'max',
        sort: [{ id: 'spill_bytes', desc: true }],
      },
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    render(
      <JotaiProvider>
        <DeepLinkBoundary
          {...BOUNDARY_PROPS}
          initialPlanId="root-plan"
          operators={DEEP_LINK_OPERATORS}
          encodedState={encoded.value}
        >
          <SerializableStateProbe />
          <OperatorSelectionProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    const value = JSON.parse(screen.getByTestId('serializable-state').textContent ?? '{}');
    expect(value).toMatchObject({
      view: {
        selection: {
          planId: 'plan-a',
          operatorNodeIds: ['operator-a', 'operator-child', 'operator-unknown'],
        },
        dag: {
          nodeColorField: 'duration_s',
          nodeColorPalette: 'viridis',
          edgeWidthField: 'bytes',
          edgeColorField: 'rows',
          edgeColorPalette: 'purple',
          nodeLabelField: 'type',
          layoutDirection: 'top-to-bottom',
          scaleType: 'linear',
        },
        dataFlow: {
          enabled: false,
          measure: 'bytes',
          labelMeasure: 'tasks',
          dimensions: ['filesystem'],
          playheadS: 25,
        },
        operatorTable: {
          groupingOrder: ['partition', 'item_type', 'item'],
          enabledGroups: ['partition', 'item_type'],
          visibleStats: ['duration_s', 'spill_bytes'],
          aggregation: 'max',
          sort: [{ id: 'spill_bytes', desc: true }],
        },
      },
      resources: {
        expandedIds: ['worker-a'],
        selectedTypes: [['worker-a', 'memory']],
        selectedFsmTypes: [['worker-a', 'task']],
        rootResourceType: 'channel',
        resourceFilter: {
          search: 'worker',
          resourceTypes: ['channel', 'memory'],
          fsmTypes: ['task', 'transfer'],
          showOthers: true,
        },
      },
    });
    expect(screen.getByTestId('operator-selection')).toHaveTextContent(
      JSON.stringify([
        {
          id: 'operator-a',
          label: 'Operator A',
          operatorIds: ['operator-a', 'operator-child'],
        },
        {
          id: 'operator-unknown',
          label: 'operator-unknown',
          operatorIds: ['operator-unknown'],
        },
      ])
    );
  });

  it('clears an existing resource filter when shared state omits resources', () => {
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }
    const store = createStore();
    store.set(resourceFilterAtom, {
      search: 'stale',
      resourceTypes: ['gpu'],
      fsmTypes: ['task'],
      showOthers: true,
    });

    render(
      <JotaiProvider store={store}>
        <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded.value}>
          <SerializableStateProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    const value = JSON.parse(screen.getByTestId('serializable-state').textContent ?? '{}');
    expect(value.resources.resourceFilter).toEqual(EMPTY_RESOURCE_FILTER);
  });

  it('exposes v3 entity state before rendering the entities route', () => {
    const entities = {
      entityType: 'task',
      resourceId: 'resource-a',
      minUsageS: 0.5,
      window: { start: 10, end: 40 },
      sortDir: 'Asc' as const,
      pageSize: 100,
      page: 2,
      selectedEntityId: 'entity-a',
    };
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'entities' },
      selection: { operatorNodeIds: ['operator-a'] },
      entities,
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) {
      return;
    }

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS} activeTab="entities" encodedState={encoded.value}>
          <InitialEntitiesProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('initial-entities')).toHaveTextContent(
      JSON.stringify({
        filters: {
          entityType: 'task',
          resourceId: 'resource-a',
          minUsageS: '0.5',
          windowStart: '10',
          windowEnd: '40',
          sortDir: 'Asc',
          pageSize: 100,
        },
        page: 2,
        selected: null,
        selectedEntityId: 'entity-a',
      })
    );
  });

  it('does not subscribe to render-time timeline hydration', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS}>
          <HydrateTimelineDuringRender />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(consoleError).not.toHaveBeenCalledWith(
      expect.stringContaining('Cannot update a component')
    );
  });

  it('shows an error toast for invalid incoming state without hydrating it', async () => {
    const selectedMessage = `Shared links support at most ${MAX_SELECTED_NODE_IDS} selected operators.`;
    const expandedMessage = `Shared links support at most ${MAX_EXPANDED_RESOURCE_IDS} expanded resources.`;
    const encoded = encodeRawState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
      selection: {
        operatorNodeIds: Array.from(
          { length: MAX_SELECTED_NODE_IDS + 1 },
          (_, index) => `operator-${index}`
        ),
      },
      resources: {
        expandedRowIds: Array.from(
          { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
          (_, index) => `resource-${index}`
        ),
      },
    });
    const toastSpy = vi.spyOn(toast, 'add');
    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded}>
            <IntakeStatusProbe />
            <ViewportProbe />
          </DeepLinkBoundary>
        </JotaiProvider>
        <Toaster />
      </>
    );

    expect(screen.getByTestId('intake-status')).toHaveTextContent('error');
    expect(screen.getByTestId('viewport')).toHaveTextContent(
      JSON.stringify({
        immediate: { start: 0, end: 0 },
        debounced: { start: 0, end: 0 },
      })
    );
    await waitFor(() => expect(toastSpy).toHaveBeenCalledTimes(2));
    expect(toastSpy).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        id: 'deep-link-intake-0',
        type: 'error',
        title: 'Could not restore shared view',
        description: expect.stringContaining(selectedMessage),
      })
    );
    expect(toastSpy).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        id: 'deep-link-intake-1',
        type: 'error',
        title: 'Could not restore shared view',
        description: expect.stringContaining(expandedMessage),
      })
    );
    await waitFor(() =>
      expect(document.querySelector('[data-slot="toast-title"]')).toHaveTextContent(
        'Could not restore shared view'
      )
    );
  });

  it('groups more than three incoming errors in an expandable summary toast', async () => {
    const encoded = encodeRawState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: -1, end: 0 } },
      selection: { planId: 'p'.repeat(129) },
      resources: {
        expandedRowIds: Array.from(
          { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
          (_, index) => `resource-${index}`
        ),
      },
    });
    const toastSpy = vi.spyOn(toast, 'add');

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded}>
            <IntakeStatusProbe />
          </DeepLinkBoundary>
        </JotaiProvider>
        <Toaster />
      </>
    );

    await waitFor(() => expect(toastSpy).toHaveBeenCalledTimes(1));
    expect(toastSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'deep-link-intake-summary',
        type: 'error',
        title: 'Could not restore shared view',
        description: null,
      })
    );

    const expand = await screen.findByText('4 errors, click to expand', { selector: 'button' });
    expect(expand).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(expand);
    const collapse = await screen.findByText('Hide details', { selector: 'button' });
    expect(collapse).toHaveAttribute('aria-expanded', 'true');
    expect(
      screen.getAllByText('Too small: expected number to be >=0 (timeline.zoomRange.start)')[0]
    ).toBeVisible();
  });

  it('copies the current viewport without changing the address bar', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    window.history.replaceState(null, '', '/profile/engine/e/query/q/timeline?unrelated=kept');

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS}>
            <SeedViewport start={20} end={60} />
            <SeedExpandedRows ids={[RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID]} />
            <SeedEmptyDataFlowDimensions />
            <SeedResourceFilter
              filter={{
                search: 'resource',
                resourceTypes: ['channel'],
                fsmTypes: [],
                showOthers: false,
              }}
            />
            <ViewportProbe />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await waitFor(() => expect(screen.getByTestId('viewport')).toHaveTextContent('"start":20'));
    const originalUrl = window.location.href;
    await userEvent.click(screen.getByRole('button', { name: 'Copy Link' }));

    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());
    expect(screen.getByRole('button', { name: 'Copy Link' }).querySelector('svg')).toHaveClass(
      'lucide-check'
    );
    const copiedUrl = writeText.mock.calls[0][0] as string;
    const parsedUrl = new URL(copiedUrl);
    const encoded = parsedUrl.searchParams.get('s');
    expect(encoded).not.toBeNull();
    expect(parsedUrl.searchParams.has('unrelated')).toBe(false);
    expect(decodeDeepLinkState(encoded!)).toEqual({
      ok: true,
      value: {
        version: 'v3',
        data: {
          route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
          timeline: { zoomRange: { start: 20, end: 60 } },
          resources: {
            expandedRowIds: [RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID],
            resourceFilter: { search: 'resource', resourceTypes: ['channel'] },
          },
        },
      },
    });
    expect(window.location.href).toBe(originalUrl);
  });

  it('includes Show All when copying an active resource filter', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS}>
            <SeedViewport start={20} end={60} />
            <SeedResourceFilter
              filter={{
                search: 'resource',
                resourceTypes: [],
                fsmTypes: [],
                showOthers: true,
              }}
            />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());

    const copiedUrl = new URL(writeText.mock.calls[0][0] as string);
    const encoded = copiedUrl.searchParams.get('s');
    expect(encoded).not.toBeNull();
    expect(decodeDeepLinkState(encoded!)).toMatchObject({
      ok: true,
      value: {
        version: 'v3',
        data: {
          resources: {
            resourceFilter: { search: 'resource', showOthers: true },
          },
        },
      },
    });
  });

  it('copies entity filters without requiring timeline state', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    window.history.replaceState(null, '', '/profile/engine/e/query/q/entities');
    const entities = {
      entityType: 'task',
      minUsageS: 0.5,
      window: { start: 10, end: 40 },
      sortDir: 'Asc' as const,
      page: 3,
      selectedEntityId: 'entity-a',
    };
    const tableState: EntitiesTableState = {
      filters: {
        entityType: 'task',
        resourceId: null,
        minUsageS: '0.5',
        windowStart: '10',
        windowEnd: '40',
        sortDir: 'Asc',
        pageSize: 50,
      },
      page: 3,
      selected: null,
      selectedEntityId: 'entity-a',
    };

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS} activeTab="entities">
            <SeedEntitiesState state={tableState} />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await userEvent.click(screen.getByRole('button', { name: 'Copy Link' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());
    const encoded = new URL(writeText.mock.calls[0][0] as string).searchParams.get('s');

    expect(decodeDeepLinkState(encoded!)).toEqual({
      ok: true,
      value: {
        version: 'v3',
        data: {
          route: { engineId: 'e', queryId: 'q', tab: 'entities' },
          entities,
        },
      },
    });
  });

  it('shows an error toast when copying fails', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error('denied')) },
    });
    const toastSpy = vi.spyOn(toast, 'add');

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS}>
            <SeedViewport start={20} end={60} />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));
    await waitFor(() =>
      expect(toastSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'deep-link-copy-error-0',
          type: 'error',
          title: 'Could not copy link',
        })
      )
    );
  });

  it('shows each generation error in a separate toast', async () => {
    const selectedMessage = `Shared links support at most ${MAX_SELECTED_NODE_IDS} selected operators.`;
    const expandedMessage = `Shared links support at most ${MAX_EXPANDED_RESOURCE_IDS} expanded resources.`;
    const selectedIds = Array.from(
      { length: MAX_SELECTED_NODE_IDS + 1 },
      (_, index) => `operator-${index}`
    );
    const expandedIds = Array.from(
      { length: MAX_EXPANDED_RESOURCE_IDS + 1 },
      (_, index) => `resource-${index}`
    );
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const toastSpy = vi.spyOn(toast, 'add');

    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS}>
            <SeedViewport start={20} end={60} />
            <SeedSelectedOperators ids={selectedIds} />
            <SeedExpandedRows ids={expandedIds} />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
        <Toaster />
      </>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));

    await waitFor(() => expect(toastSpy).toHaveBeenCalledTimes(2));
    expect(toastSpy).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        id: 'deep-link-copy-error-0',
        type: 'error',
        title: 'Could not copy link',
        description: expect.stringContaining(selectedMessage),
      })
    );
    expect(toastSpy).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        id: 'deep-link-copy-error-1',
        type: 'error',
        title: 'Could not copy link',
        description: expect.stringContaining(expandedMessage),
      })
    );
    expect(writeText).not.toHaveBeenCalled();
  });
});

describe('DeepLinkBoundary pinned rows', () => {
  const LANE_ID = `__long_entities__${RESOURCE_A_ID}`;

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function encodeState(resources?: { pinnedRowIds: string[] }) {
    const encoded = encodeDeepLinkState({
      route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 40 } },
      ...(resources ? { resources } : {}),
    });
    if (!encoded.ok) {
      throw new Error('Failed to encode the test link');
    }
    return encoded.value;
  }

  function mockClipboard() {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    return writeText;
  }

  it('restores pinned rows from a shared link', () => {
    render(
      <JotaiProvider>
        <DeepLinkBoundary
          {...BOUNDARY_PROPS}
          encodedState={encodeState({ pinnedRowIds: [RESOURCE_B_ID, LANE_ID] })}
        >
          <PinnedRowsProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('pinned-rows')).toHaveTextContent(
      JSON.stringify([RESOURCE_B_ID, LANE_ID])
    );
  });

  it('replaces pins already in the session with the ones in the link', () => {
    const store = createStore();
    store.set(pinnedIdsAtom, new Set([RESOURCE_A_ID]));
    expect([...store.get(pinnedIdsAtom)]).toEqual([RESOURCE_A_ID]);

    render(
      <JotaiProvider store={store}>
        <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encodeState()}>
          <PinnedRowsProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('pinned-rows')).toHaveTextContent('[]');
  });

  it('copies pinned rows into the shared link', async () => {
    const writeText = mockClipboard();

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS}>
          <SeedViewport start={20} end={60} />
          <SeedPinnedRows ids={[RESOURCE_B_ID, RESOURCE_A_ID]} />
          <CopyLinkButton />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());

    const encoded = new URL(writeText.mock.calls[0][0] as string).searchParams.get('s');
    expect(decodeDeepLinkState(encoded!)).toMatchObject({
      ok: true,
      value: { data: { resources: { pinnedRowIds: [RESOURCE_B_ID, RESOURCE_A_ID] } } },
    });
  });

  it('leaves pins out of the link when nothing is pinned', async () => {
    const writeText = mockClipboard();

    render(
      <JotaiProvider>
        <DeepLinkBoundary {...BOUNDARY_PROPS}>
          <SeedViewport start={20} end={60} />
          <CopyLinkButton />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());

    const encoded = new URL(writeText.mock.calls[0][0] as string).searchParams.get('s');
    const decoded = decodeDeepLinkState(encoded!);
    expect(decoded.ok && 'resources' in decoded.value.data).toBe(false);
  });
});
