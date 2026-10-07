// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, fireEvent, render, screen } from '@testing-library/react';
import { Provider } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import {
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSelectedOperatorsData,
} from '@quent/hooks';
import type { Operator } from '@quent/utils';
import { DAGNodeInfoPanel } from '../dag/DAGNodeInfoPanel';
import { QueryToolbar } from '../timeline/QueryToolbar';
import { OperatorGanttChart } from './OperatorGanttChart';
import type { OperatorActiveSpanEntry } from './types';

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
