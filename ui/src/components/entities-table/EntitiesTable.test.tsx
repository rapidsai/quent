// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createStore, Provider } from 'jotai';
import { useSetSelectedNodeIds } from '@quent/hooks';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { EntitiesTable } from './EntitiesTable';

const useEntities = vi.fn();

vi.mock('@quent/client', () => ({
  useEntities: (...args: unknown[]) => useEntities(...args),
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
    fsm_types: { Task: {} },
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
      data: { items: [{ entity: fsm, usage_duration_s: 0.75 }], total: 1 },
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
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.change(screen.getByLabelText('Page size'), { target: { value: '1.5' } });
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.page.max).toBe(1);
  });

  it('clears selected entity details when a filter changes', () => {
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByText('Entity 1'));
    expect(screen.getByText('running')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Min usage (s)'), { target: { value: '0.5' } });

    expect(screen.queryByText('running')).not.toBeInTheDocument();
    expect(screen.getByText('Select an entity to view its states.')).toBeInTheDocument();
  });

  it('preserves selected entity details for page-size changes', () => {
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByText('Entity 1'));
    fireEvent.change(screen.getByLabelText('Page size'), { target: { value: '100' } });

    expect(screen.getByText('running')).toBeInTheDocument();
  });

  it('blocks invalid time windows before fetching', () => {
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.change(screen.getByLabelText('Window start (s)'), { target: { value: '11' } });

    expect(screen.getByRole('alert')).toHaveTextContent('Window start must not exceed window end.');
    expect(useEntities.mock.lastCall?.[1]).toEqual({ enabled: false });
  });

  it('shows usage and state durations and supports keyboard row selection', () => {
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    expect(screen.getByText('Longest usage')).toBeInTheDocument();
    expect(screen.getByText('750.00ms')).toBeInTheDocument();

    fireEvent.keyDown(screen.getByRole('row', { name: /Entity 1/ }), { key: 'Enter' });

    expect(screen.getByText(/for 1.00s/)).toBeInTheDocument();
  });

  it('searches operators and resets active filters', () => {
    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    fireEvent.click(screen.getByRole('combobox', { name: 'Operator' }));
    fireEvent.change(screen.getByLabelText('Search operator'), { target: { value: 'one' } });
    fireEvent.click(screen.getByRole('option', { name: 'Operator One' }));

    expect(screen.getByText('1 active filter')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual([]);
  });

  it('dims existing rows while replacement data is pending', () => {
    useEntities.mockReturnValue({
      data: { items: [{ entity: fsm, usage_duration_s: 0.75 }], total: 1 },
      isLoading: false,
      isFetching: true,
      isError: false,
      error: null,
    });

    render(<EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />);

    const tableContainer = screen.getByRole('table').closest('[aria-busy]');
    expect(tableContainer).toHaveAttribute('aria-busy', 'true');
    expect(tableContainer).toHaveClass('opacity-60');
    expect(screen.getByText('Entity 1')).toBeInTheDocument();
  });

  it('uses the selected DAG operator as the entity filter', () => {
    const store = createStore();
    render(
      <Provider store={store}>
        <DagSelectionControl />
        <EntitiesTable engineId="engine-1" queryId="query-1" queryBundle={queryBundle} />
      </Provider>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Select DAG operator' }));
    act(() => vi.advanceTimersByTime(300));

    const params = useEntities.mock.lastCall?.[0];
    expect(params.request.entry.application.operator_ids).toEqual(['operator-1']);
    expect(screen.getByRole('combobox', { name: 'Operator' })).toHaveTextContent('Operator One');
  });
});
