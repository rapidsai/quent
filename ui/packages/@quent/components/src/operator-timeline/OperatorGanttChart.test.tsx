// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import {
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSelectedOperatorsData,
} from '@quent/hooks';
import type { EntityRef, Operator, QueryBundle } from '@quent/utils';
import { DAGNodeInfoPanel } from '../dag/DAGNodeInfoPanel';
import { QueryPlanNode } from '../query-plan/QueryPlanNode';
import { QueryToolbar } from '../timeline/QueryToolbar';
import { OperatorGanttChart } from './OperatorGanttChart';
import type { OperatorActiveSpanEntry } from './types';
import { operatorsWithActiveSpans, operatorsWithActiveSpansForWorker } from './utils';

const mocks = vi.hoisted(() => ({
  ganttChart: vi.fn(),
}));

vi.mock('../timeline/timelineEchartsTheme', () => ({
  useTimelineEchartsTheme: () => ({ textColor: '#000000' }),
}));

vi.mock('../gantt-chart/GanttChart', () => ({
  GanttChart: (props: { onEvents: { click: (params: unknown) => void } }) => {
    mocks.ganttChart(props);
    return null;
  },
}));

function makeOperator(id: string, parentOperatorIds: string[] = []): Operator {
  return {
    id,
    plan_id: 'plan',
    parent_operator_ids: parentOperatorIds,
    instance_name: id,
    operator_type_name: 'test',
    custom_attributes: [],
    statistics: null,
    active_span: null,
  };
}

function SelectionControls() {
  const selection = useOperatorSelection();
  const selectedIds = useSelectedOperatorIds();
  const selectedOperatorsData = useSelectedOperatorsData();
  const updateSelection = useOperatorSelectionActions();

  return (
    <>
      <button
        type="button"
        onClick={() =>
          updateSelection({
            type: 'add',
            selectionId: 'parent',
            label: 'parent',
            operatorIds: ['parent', 'left', 'right'],
            selectedData: {
              nodeId: 'parent',
              label: 'parent',
              operationType: 'test',
              statistics: [],
              relatedOperators: [
                {
                  nodeId: 'left',
                  label: 'left',
                  operationType: 'test',
                  statistics: [],
                },
                {
                  nodeId: 'right',
                  label: 'right',
                  operationType: 'test',
                  statistics: [],
                },
              ],
            },
          })
        }
      >
        Select parent
      </button>
      <output data-testid="selection-ids">{JSON.stringify([...selectedIds].sort())}</output>
      <output data-testid="selection-groups">
        {JSON.stringify([...selection.selections.keys()].sort())}
      </output>
      <output data-testid="selected-operators">
        {JSON.stringify(selectedOperatorsData.map(operator => operator.nodeId).sort())}
      </output>
    </>
  );
}

describe('OperatorGanttChart', () => {
  it.each(['plan', 'worker'])(
    'shows ordered attributes when an operator is selected from the %s timeline',
    scope => {
      const operator: Operator = {
        ...makeOperator('scan'),
        active_span: { start: 0, end: 1 },
        custom_attributes: [
          { key: 'source', value: 'First input' },
          { key: 'Configuration', value: [{ key: 'expression', value: 'price * discount' }] },
          { key: 'source', value: 'Second input' },
        ],
        statistics: { custom_statistics: [{ value: { key: 'rows', value: 0 }, quantity: null }] },
      };
      const bundle = {
        entities: { operators: { scan: operator } },
        plan_tree: { id: 'plan', worker: 'worker', children: [] },
      } as unknown as QueryBundle<EntityRef>;
      const operators =
        scope === 'plan'
          ? operatorsWithActiveSpans(bundle, 'plan')
          : operatorsWithActiveSpansForWorker(bundle, 'worker');

      render(
        <Provider store={createStore()}>
          <DAGNodeInfoPanel />
          <OperatorGanttChart
            operators={operators}
            allOperators={[operator]}
            durationSeconds={1}
            isDark={false}
          />
        </Provider>
      );
      act(() => {
        mocks.ganttChart.mock.lastCall?.[0].onEvents.click({
          dataIndex: 0,
          seriesName: 'operator-span',
        });
      });

      const attributes = screen.getByRole('heading', { name: 'Attributes' }).closest('section')!;
      expect(
        within(attributes)
          .getAllByRole('definition')
          .map(value => value.textContent)
      ).toEqual(['First input', 'price * discount', 'Second input']);
      expect(
        within(attributes)
          .getAllByRole('term')
          .map(term => term.textContent)
      ).toEqual(['source:', 'expression:', 'source:']);
      expect(within(attributes).getByRole('heading', { name: 'Configuration' })).toBeVisible();
      const statistics = screen.getByRole('heading', { name: 'Statistics' }).closest('section')!;
      expect(within(statistics).getByText('rows:')).toBeVisible();
      expect(within(statistics).getByText('0')).toBeVisible();
    }
  );

  it('selects the DAG group from a bar, replaces other selections, and toggles it off', () => {
    const parent = makeOperator('parent');
    const child = makeOperator('child', ['parent']);
    const other = makeOperator('other');
    const operators: OperatorActiveSpanEntry[] = [parent, other].map((operator, index) => ({
      operatorId: operator.id,
      label: operator.id,
      typeName: 'test',
      startMs: 0,
      endMs: 1,
      rowIndex: index,
      planId: 'plan',
      statistics: [],
    }));
    render(
      <Provider store={createStore()}>
        <SelectionControls />
        <DAGNodeInfoPanel />
        <div data-testid="parent-node">
          <QueryPlanNode
            data={{
              nodeId: 'parent',
              label: 'parent',
              operationType: 'test',
              metadata: {
                rawNode: parent,
                relatedOperatorIds: ['child'],
                relatedOperators: [child],
              },
            }}
          />
        </div>
        <div data-testid="other-node">
          <QueryPlanNode
            data={{
              nodeId: 'other',
              label: 'other',
              operationType: 'test',
              metadata: { rawNode: other },
            }}
          />
        </div>
        <OperatorGanttChart
          operators={operators}
          allOperators={[parent, child, other]}
          durationSeconds={1}
          isDark={false}
        />
      </Provider>
    );
    const clickBar = (dataIndex: number) =>
      act(() => {
        mocks.ganttChart.mock.lastCall?.[0].onEvents.click({
          dataIndex,
          seriesName: 'operator-span',
        });
      });
    const parentNode = screen.getByTestId('parent-node');
    const otherNode = screen.getByTestId('other-node');

    clickBar(1);
    expect(otherNode.querySelector('.shadow-glow')).not.toBeNull();
    clickBar(0);
    expect(screen.getByTestId('selection-ids')).toHaveTextContent(
      JSON.stringify(['child', 'parent'])
    );
    expect(
      screen.getByRole('group', { name: 'Operator Gantt chart: parent, other' })
    ).toHaveAttribute('data-selected-operator-ids', 'parent');
    expect(parentNode.querySelector('.shadow-glow')).not.toBeNull();
    expect(otherNode.querySelector('.opacity-35')).not.toBeNull();
    expect(screen.getByTestId('operator-accordion-parent')).toBeVisible();
    expect(screen.queryByTestId('operator-accordion-other')).not.toBeInTheDocument();

    clickBar(0);
    expect(screen.getByTestId('selection-ids')).toHaveTextContent('[]');
    expect(parentNode.querySelector('.shadow-glow')).toBeNull();
    expect(parentNode.querySelector('.opacity-100')).not.toBeNull();
    expect(otherNode.querySelector('.opacity-100')).not.toBeNull();
    expect(screen.queryByTestId('operator-accordion-parent')).not.toBeInTheDocument();
  });

  it('splits a selected parent when a covered child is deselected', () => {
    const allOperators = [
      makeOperator('parent'),
      makeOperator('left', ['parent']),
      makeOperator('right', ['parent']),
    ];
    const operators: OperatorActiveSpanEntry[] = [
      {
        operatorId: 'left',
        label: 'left',
        typeName: 'test',
        startMs: 0,
        endMs: 1,
        rowIndex: 0,
        planId: 'plan',
        statistics: [],
      },
    ];

    render(
      <Provider>
        <SelectionControls />
        <QueryToolbar />
        <DAGNodeInfoPanel />
        <OperatorGanttChart
          operators={operators}
          allOperators={allOperators}
          durationSeconds={1}
          isDark={false}
        />
      </Provider>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Select parent' }));
    act(() => {
      mocks.ganttChart.mock.lastCall?.[0].onEvents.click({
        dataIndex: 0,
        seriesName: 'operator-span',
      });
    });

    expect(screen.getByTestId('selection-ids')).toHaveTextContent(JSON.stringify(['right']));
    expect(screen.getByTestId('selection-groups')).toHaveTextContent(JSON.stringify(['right']));
    expect(screen.getByTestId('selected-operators')).toHaveTextContent(JSON.stringify(['right']));
    expect(screen.queryByRole('button', { name: 'Remove parent' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Remove right' })).toBeInTheDocument();
    expect(screen.getByTestId('operator-accordion-right')).toBeInTheDocument();
    expect(screen.queryByTestId('operator-accordion-parent')).not.toBeInTheDocument();
  });
});
