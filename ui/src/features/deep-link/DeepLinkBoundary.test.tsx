// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect } from 'react';
import { Provider as JotaiProvider, useAtomValue, useSetAtom } from 'jotai';
import {
  useDebouncedZoomRange,
  useHydrateTimelineAtoms,
  useSerializableViewState,
  useSetDebouncedZoomRange,
  useSetZoomRange,
  useZoomRange,
} from '@quent/hooks';
import { NVTX_SECTION_ID, toast, Toaster } from '@quent/components';
import { render, screen, waitFor, userEvent } from '@/test/test-utils';
import { entitiesTableStateAtom, type EntitiesTableState } from '@/atoms/entitiesTable';
import {
  expandedIdsAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import {
  OPERATOR_TABLE_INDEX_ORDER,
  OPERATOR_TABLE_PERSIST_KEY,
} from '@/components/operator-table/types';
import { CopyLinkButton } from './CopyLinkButton';
import { DeepLinkBoundary } from './DeepLinkBoundary';
import { decodeDeepLinkState, encodeDeepLinkState } from './deepLink.codec';
import { DEEP_LINK_NAV_SLOT_ID } from './deepLink.constants';
import { useDeepLink } from './deepLink.context';

const BOUNDARY_PROPS = {
  engineId: 'e',
  queryId: 'q',
  activeTab: 'timeline' as const,
  durationSeconds: 100,
  isQueryReady: true,
};

const RESOURCE_A_ID = '01a025ff-ea8b-7881-9d31-72a275872c9d';
const RESOURCE_B_ID = '01a025ff-ea8b-7881-9d31-72a275872c9e';

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
  });
  const expandedIds = useAtomValue(expandedIdsAtom);
  const selectedTypes = useAtomValue(selectedTypesAtom);
  const selectedFsmTypes = useAtomValue(selectedFsmTypesAtom);
  const rootResourceType = useAtomValue(rootResourceTypeAtom);
  return (
    <output data-testid="serializable-state">
      {JSON.stringify({
        view: read(),
        resources: {
          expandedIds: [...expandedIds].sort(),
          selectedTypes: [...selectedTypes],
          selectedFsmTypes: [...selectedFsmTypes],
          rootResourceType,
        },
      })}
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

function SeedEmptyDataFlowDimensions() {
  const { hydrate } = useSerializableViewState({
    operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
    operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
  });

  useLayoutEffect(() => {
    hydrate({ dataFlow: { dimensions: [] } });
  }, [hydrate]);
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
      selection: { planId: 'plan-a', operatorNodeIds: ['operator-a'] },
      resources: {
        expandedRowIds: ['worker-a'],
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
        <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState={encoded.value}>
          <SerializableStateProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    const value = JSON.parse(screen.getByTestId('serializable-state').textContent ?? '{}');
    expect(value).toMatchObject({
      view: {
        selection: { planId: 'plan-a', operatorNodeIds: ['operator-a'] },
        dag: {
          nodeColorField: 'duration_s',
          nodeColorPalette: 'viridis',
          edgeWidthField: 'bytes',
          edgeColorField: 'rows',
          edgeColorPalette: 'purple',
          nodeLabelField: 'type',
          layoutDirection: 'top-to-bottom',
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
      },
    });
  });

  it('exposes v3 entity state before rendering the entities route', () => {
    const entities = {
      operatorId: null,
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
        manualOperatorOverride: { dagOperatorId: 'operator-a', value: null },
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
    const toastSpy = vi.spyOn(toast, 'add');
    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS} encodedState="v1.invalid">
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
    await waitFor(() =>
      expect(toastSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'deep-link-intake',
          type: 'error',
          title: 'Could not restore shared view',
        })
      )
    );
    await waitFor(() =>
      expect(document.querySelector('[data-slot="toast-title"]')).toHaveTextContent(
        'Could not restore shared view'
      )
    );
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
        <div id={DEEP_LINK_NAV_SLOT_ID} />
        <JotaiProvider>
          <DeepLinkBoundary {...BOUNDARY_PROPS}>
            <SeedViewport start={20} end={60} />
            <SeedExpandedRows ids={[RESOURCE_B_ID, NVTX_SECTION_ID, RESOURCE_A_ID]} />
            <SeedEmptyDataFlowDimensions />
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
          resources: { expandedRowIds: [RESOURCE_A_ID, RESOURCE_B_ID, NVTX_SECTION_ID] },
        },
      },
    });
    expect(window.location.href).toBe(originalUrl);
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
      manualOperatorOverride: null,
      page: 3,
      selected: null,
      selectedEntityId: 'entity-a',
    };

    render(
      <>
        <div id={DEEP_LINK_NAV_SLOT_ID} />
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
        <div id={DEEP_LINK_NAV_SLOT_ID} />
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
          id: 'deep-link-copy-error',
          type: 'error',
          title: 'Could not copy link',
        })
      )
    );
  });
});
