// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createStore, Provider } from 'jotai';
import {
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedNodeIds,
} from '@quent/hooks';
import type { EntityRef, Operator, QueryBundle } from '@quent/utils';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { EntitiesTable } from './EntitiesTable';

function renderTable(
  ui: React.ReactElement,
  options: { store?: ReturnType<typeof createStore> } = {}
) {
  const { store } = options;
  const content = store ? <Provider store={store}>{ui}</Provider> : ui;
  return render(<ThemeProvider>{content}</ThemeProvider>);
}

const useEntities = vi.fn();
const useEntityList = vi.fn();

vi.mock('@quent/client', () => ({
  useEntities: (...args: unknown[]) => useEntities(...args),
  useEntityList: (...args: unknown[]) => useEntityList(...args),
}));

function makeOperator(
  id: string,
  instanceName: string,
  typeName: string,
  parentOperatorIds: string[] = []
): Operator {
  return {
    id,
    plan_id: null,
    parent_operator_ids: parentOperatorIds,
    instance_name: instanceName,
    operator_type_name: typeName,
    custom_attributes: {},
    observations: [],
    statistics: null,
    active_span: null,
  };
}

const queryBundle = {
  query_id: 'query-1',
  duration_s: 10,
  entities: {
    engine: {
      id: 'engine-1',
      start_time_unix_ns: null,
      duration_s: null,
      instance_name: null,
      implementation: null,
    },
    query_group: {
      id: 'query-group-1',
      instance_name: null,
      engine_id: 'engine-1',
    },
    query: {
      id: 'query-1',
      query_group_id: 'query-group-1',
      instance_name: null,
      start_unix_ns: null,
      planning_s: null,
      executing_s: null,
      completed_s: null,
    },
    workers: {},
    plans: {},
    operators: {
      'operator-1': makeOperator('operator-1', 'Operator One', 'Scan'),
    },
    ports: {},
    resource_types: {},
    resource_group_types: {},
    resources: {
      'resource-1': {
        id: 'resource-1',
        instance_name: 'GPU 1',
        type_name: 'GPU',
        parent_group_id: '',
      },
    },
    resource_groups: {},
    fsm_types: { Task: { name: 'Task', states: [], transitions: [] } },
  },
  plan_tree: { id: 'plan-1', worker: null, children: [] },
  resource_tree: { Resource: { Resource: 'resource-1' } },
  unique_operator_names: [],
  quantity_specs: {},
  start_time_unix_ns: 0n,
} satisfies QueryBundle<EntityRef>;

function withOperators(
  operators: QueryBundle<EntityRef>['entities']['operators']
): QueryBundle<EntityRef> {
  return {
    ...queryBundle,
    entities: {
      ...queryBundle.entities,
      operators,
    },
  };
}

const logicalOperatorQueryBundle = withOperators({
  logical: makeOperator('logical', 'Logical Operator', 'Logical'),
  'child-one': makeOperator('child-one', 'Child One', 'Physical', ['logical']),
  'child-two': makeOperator('child-two', 'Child Two', 'Physical', ['logical']),
});

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
  const updateOperatorSelection = useOperatorSelectionActions();
  return (
    <button
      type="button"
      onClick={() =>
        updateOperatorSelection({
          type: 'add',
          selectionId: 'operator-1',
          label: 'Operator One',
          operatorIds: ['operator-1'],
          inspectedData: {
            nodeId: 'operator-1',
            label: 'Operator One',
            operationType: 'scan',
            statistics: [],
          },
        })
      }
    >
      Select DAG operator
    </button>
  );
}

function OperatorSelectionProbe() {
  const operatorIds = useSelectedNodeIds();
  const selection = useOperatorSelection();
  return (
    <>
      <output data-testid="selected-operator-ids">{JSON.stringify([...operatorIds].sort())}</output>
      <output data-testid="operator-selection-labels">
        {JSON.stringify([...selection.selections.values()].map(value => value.label).sort())}
      </output>
    </>
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
    fireEvent.change(screen.getByPlaceholderText('Search operators…'), {
      target: { value: 'one' },
    });
    fireEvent.click(screen.getByRole('option', { name: 'Operator One' }));

    expect(screen.getByText('1 active filter')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual([]);
  });

  it('supports selecting multiple operators from the dropdown', () => {
    const store = createStore();
    const multiOperatorQueryBundle = withOperators({
      ...queryBundle.entities.operators,
      'operator-2': makeOperator('operator-2', 'Operator Two', 'Filter'),
    });

    renderTable(
      <>
        <OperatorSelectionProbe />
        <EntitiesTable
          engineId="engine-1"
          queryId="query-1"
          queryBundle={multiOperatorQueryBundle}
        />
      </>,
      { store }
    );

    fireEvent.click(screen.getByRole('combobox', { name: 'Operator' }));
    fireEvent.click(screen.getByRole('option', { name: 'Operator One' }));
    fireEvent.click(screen.getByRole('option', { name: 'Operator Two' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual(
      expect.arrayContaining(['operator-1', 'operator-2'])
    );
    expect(params.request.entry.application.operator_ids).toHaveLength(2);
    expect(screen.getByRole('combobox', { name: 'Operator' })).toHaveTextContent('2 selected');
    expect(screen.getByTestId('operator-selection-labels')).toHaveTextContent(
      JSON.stringify(['Operator One', 'Operator Two'])
    );
  });

  it('groups logical operators and splits the group when a child is deselected', () => {
    const store = createStore();
    renderTable(
      <>
        <OperatorSelectionProbe />
        <EntitiesTable
          engineId="engine-1"
          queryId="query-1"
          queryBundle={logicalOperatorQueryBundle}
        />
      </>,
      { store }
    );

    fireEvent.click(screen.getByRole('combobox', { name: 'Operator' }));
    fireEvent.click(screen.getByRole('option', { name: 'Logical Operator' }));

    let params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual(
      expect.arrayContaining(['logical', 'child-one', 'child-two'])
    );
    expect(params.request.entry.application.operator_ids).toHaveLength(3);
    expect(screen.getByTestId('selected-operator-ids')).toHaveTextContent(
      JSON.stringify(['child-one', 'child-two', 'logical'])
    );
    expect(screen.getByTestId('operator-selection-labels')).toHaveTextContent(
      JSON.stringify(['Logical Operator'])
    );

    fireEvent.click(screen.getByRole('option', { name: 'Child One' }));

    params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual(
      expect.arrayContaining(['logical', 'child-two'])
    );
    expect(params.request.entry.application.operator_ids).toHaveLength(2);
    expect(screen.getByTestId('operator-selection-labels')).toHaveTextContent(
      JSON.stringify(['Child Two', 'Logical Operator'])
    );
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

  it('clamps min usage filter when the longest-entity bound narrows after loading', () => {
    useEntityList.mockReturnValue({
      data: undefined,
      isLoading: true,
      isFetching: true,
      isError: false,
      error: null,
    });

    const { rerender } = renderTable(
      <EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />
    );

    fireEvent.change(screen.getByLabelText('Min usage (s)'), { target: { value: '8' } });
    act(() => vi.advanceTimersByTime(300));

    expect(useEntities.mock.lastCall?.[0].request.entry.filter.min_usage_s).toBe(8);

    useEntityList.mockReturnValue({
      data: { items: [{ entity: fsm, usage_duration_s: 3 }], total: 1 },
      isLoading: false,
      isFetching: false,
      isError: false,
      error: null,
    });

    rerender(
      <ThemeProvider>
        <EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />
      </ThemeProvider>
    );
    act(() => vi.advanceTimersByTime(300));

    expect(screen.getByLabelText('Min usage (s)')).toHaveValue(3);
    expect(useEntities.mock.lastCall?.[0].request.entry.filter.min_usage_s).toBe(3);
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
});
