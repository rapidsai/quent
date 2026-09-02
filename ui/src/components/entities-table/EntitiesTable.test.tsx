// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createStore, Provider } from 'jotai';
import { StrictMode } from 'react';
import { useSetSelectedNodeIds } from '@quent/hooks';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { entitiesTableStateAtom } from '@/atoms/entitiesTable';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { EntitiesTable } from './EntitiesTable';

function renderTable(
  ui: React.ReactElement,
  options: { store?: ReturnType<typeof createStore> } = {}
) {
  const store = options.store ?? createStore();
  const content = <Provider store={store}>{ui}</Provider>;
  return render(<ThemeProvider>{content}</ThemeProvider>);
}

const useEntities = vi.fn();
const useEntityList = vi.fn();

vi.mock('@quent/client', () => ({
  useEntities: (...args: unknown[]) => useEntities(...args),
  useEntityList: (...args: unknown[]) => useEntityList(...args),
}));

const queryBundle = {
  query_id: 'query-1',
  duration_s: 10,
  entities: {
    operators: {
      'operator-1': {
        id: 'operator-1',
        instance_name: 'Operator One',
        operator_type_name: 'Scan',
      },
    },
    resources: {
      'resource-1': {
        id: 'resource-1',
        instance_name: 'GPU 1',
        type_name: 'GPU',
      },
    },
    fsm_types: { Task: { name: 'Task', states: [], transitions: [] } },
  },
} as unknown as QueryBundle<EntityRef>;

const fsm = {
  id: 'entity-1',
  type_name: 'Task',
  instance_name: 'Entity 1',
  transitions: [
    {
      name: 'running',
      timestamp: 0,
      usages: [],
      attributes: [],
      derived_attributes: [],
    },
    {
      name: 'finished',
      timestamp: 1,
      usages: [],
      attributes: [],
      derived_attributes: [],
    },
  ],
};

function DagSelectionControl() {
  const setSelectedNodeIds = useSetSelectedNodeIds();
  return (
    <button type="button" onClick={() => setSelectedNodeIds(new Set(['operator-1']))}>
      Select DAG operator
    </button>
  );
}

describe('EntitiesTable', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useEntities.mockReset();
    useEntities.mockReturnValue({
      data: { items: [{ entity: fsm, usage_duration_s: 0.25 }], total: 1 },
      isLoading: false,
      isFetching: false,
      isError: false,
      error: null,
    });
    useEntityList.mockReset();
    useEntityList.mockReturnValue({
      data: { items: [{ entity: fsm, usage_duration_s: 0.25 }], total: 1 },
      isLoading: false,
      isFetching: false,
      isError: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('normalizes page size before sending it to the API', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.change(screen.getByLabelText('Page size'), { target: { value: '1.5' } });
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.page.max).toBe(1);
  });

  it('clears selected entity details when a filter changes', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByText('Entity 1'));
    expect(screen.getByText('running')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Min usage (s)'), { target: { value: '0.5' } });

    expect(screen.queryByText('running')).not.toBeInTheDocument();
    expect(screen.getByText('Select an entity to view its states.')).toBeInTheDocument();
  });

  it('deselects an entity when its selected row is clicked again', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    const row = screen.getByRole('row', { name: /Entity 1/ });
    fireEvent.click(row);
    expect(screen.getByText('running')).toBeInTheDocument();

    fireEvent.click(row);

    expect(screen.queryByText('running')).not.toBeInTheDocument();
    expect(screen.getByText('Select an entity to view its states.')).toBeInTheDocument();
  });

  it('preserves selected entity details for page-size changes', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByText('Entity 1'));
    fireEvent.change(screen.getByLabelText('Page size'), { target: { value: '100' } });

    expect(screen.getByText('running')).toBeInTheDocument();
  });

  it('blocks invalid time windows before fetching', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.change(screen.getByLabelText('Window start (s)'), { target: { value: '11' } });

    expect(screen.getByRole('alert')).toHaveTextContent('Window start must not exceed window end.');
    expect(useEntities.mock.lastCall?.[1]).toEqual({ enabled: false });
  });

  it('shows usage and state durations and supports keyboard row selection', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    expect(screen.getByText('Longest usage')).toBeInTheDocument();
    expect(screen.getAllByText('1.00s').length).toBeGreaterThan(0);
    expect(screen.getByText('250.00ms')).toBeInTheDocument();

    fireEvent.keyDown(screen.getByRole('row', { name: /Entity 1/ }), { key: 'Enter' });

    expect(screen.getByText('Total span')).toBeInTheDocument();
  });

  it('searches operators and resets active filters', () => {
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByRole('combobox', { name: 'Operator' }));
    fireEvent.change(screen.getByLabelText('Search operator'), { target: { value: 'one' } });
    fireEvent.click(screen.getByRole('option', { name: 'Operator One' }));

    expect(screen.getByText('1 active filter')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual([]);
  });

  it('shows empty state when the response contains no entities', () => {
    useEntities.mockReturnValue({
      data: { items: [], total: 0 },
      isLoading: false,
      isFetching: false,
      isError: false,
      error: null,
    });

    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    expect(screen.getByText('No entities match the filters.')).toBeInTheDocument();
  });

  it('shows error state when the query fails', () => {
    useEntities.mockReturnValue({
      data: undefined,
      isLoading: false,
      isFetching: false,
      isError: true,
      error: new Error('network timeout'),
    });

    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    expect(screen.getByText('Failed to load entities: network timeout')).toBeInTheDocument();
  });

  it('shows a loading overlay over existing rows while replacement data is pending', () => {
    useEntities.mockReturnValue({
      data: { items: [{ entity: fsm, usage_duration_s: 1 }], total: 1 },
      isLoading: false,
      isFetching: true,
      isError: false,
      error: null,
    });

    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    const tableContainer = screen.getByRole('table').closest('[aria-busy]');
    expect(tableContainer).toHaveAttribute('aria-busy', 'true');
    expect(screen.getByRole('status')).toHaveTextContent('Updating…');
    expect(screen.getByText('Entity 1')).toBeInTheDocument();
  });

  it('uses the selected DAG operator as the entity filter', () => {
    const store = createStore();
    renderTable(
      <>
        <DagSelectionControl />
        <EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />
      </>,
      { store }
    );

    fireEvent.click(screen.getByRole('button', { name: 'Select DAG operator' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual(['operator-1']);
    expect(screen.getByRole('combobox', { name: 'Operator' })).toHaveTextContent('Operator One');
  });

  it('reads entity controls from the Jotai table state', () => {
    const store = createStore();
    store.set(entitiesTableStateAtom, {
      filters: {
        entityType: 'Task',
        resourceId: 'resource-1',
        minUsageS: '0.5',
        windowStart: '1',
        windowEnd: '8',
        sortDir: 'Asc',
        pageSize: 100,
      },
      manualOperatorOverride: { dagOperatorId: null, value: 'operator-1' },
      page: 0,
      selected: null,
      selectedEntityId: 'entity-1',
    });
    renderTable(
      <StrictMode>
        <EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />
      </StrictMode>,
      { store }
    );

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry).toMatchObject({
      window: { start: 1, end: 8 },
      filter: {
        scope: { Resource: { resource_id: 'resource-1' } },
        entity_type_name: 'Task',
        min_usage_s: 0.5,
      },
      sort: { key: 'UsageDuration', dir: 'Asc' },
      page: { max: 100, page: 0 },
      application: { operator_ids: ['operator-1'] },
    });
    expect(screen.getByText('running')).toBeInTheDocument();
  });

  it('writes entity control changes to the Jotai table state', () => {
    const store = createStore();
    renderTable(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />, {
      store,
    });

    fireEvent.change(screen.getByLabelText('Min usage (s)'), { target: { value: '0.75' } });

    expect(store.get(entitiesTableStateAtom).filters?.minUsageS).toBe('0.75');
  });
});
