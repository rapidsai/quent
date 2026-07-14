// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { Provider } from 'jotai';
import { render, screen, fireEvent } from '@testing-library/react';
import { useDataFlowSync } from '@quent/hooks';
import { DagPlayhead, NodeFlowBar } from '@quent/components';
import type { DataFlowTimelineResponse, EntityRef, QueryBundle } from '@quent/utils';

// 4 bins of 2s over [0, 8): op-1 totals per bin are [1, 3, 5, 0].
const RESPONSE: DataFlowTimelineResponse = {
  Binned: {
    config: { span: { start: 0, end: 8 }, bin_duration: 2, num_bins: BigInt(4) },
    decl: {
      entity_type_name: 'Task',
      dimension_name: 'Data location',
      dimension_keys: [
        { key: 'memory', display_name: 'Memory' },
        { key: 'filesystem', display_name: 'Filesystem' },
      ],
      measures: [{ name: 'tasks', display_name: 'Tasks', quantity: 'unit', kind: 'Occupancy' }],
    },
    operators: {
      'op-1': {
        values: {
          tasks: {
            queueing: { memory: [1, 2, 0, 0] },
            computing: { memory: [0, 1, 3, 0], filesystem: [0, 0, 2, 0] },
          },
        },
      },
    },
  },
};

const QUERY_BUNDLE = {
  entities: {
    fsm_types: {
      Task: {
        name: 'Task',
        states: [
          { name: 'queueing', usages: [] },
          { name: 'computing', usages: [] },
        ],
        transitions: [],
      },
    },
  },
  quantity_specs: {
    unit: {
      symbol: '',
      singular: 'task',
      plural: 'tasks',
      occupancy_prefix: 'None',
      rate_prefix: 'None',
    },
  },
} as unknown as QueryBundle<EntityRef>;

function Harness({ response }: { response: DataFlowTimelineResponse }) {
  useDataFlowSync({ response, queryBundle: QUERY_BUNDLE });
  return (
    <>
      <DagPlayhead startTimeUnixNs={BigInt(0)} />
      <NodeFlowBar operatorId="op-1" isDark={false} />
    </>
  );
}

function renderOverlay(response: DataFlowTimelineResponse) {
  return render(
    <Provider>
      <Harness response={response} />
    </Provider>
  );
}

describe('data-flow overlay components', () => {
  it('renders nothing when the response is "Unsupported"', () => {
    renderOverlay('Unsupported');
    expect(screen.queryByTestId('dag-playhead')).not.toBeInTheDocument();
    expect(screen.queryByTestId('node-flow-bar')).not.toBeInTheDocument();
  });

  it('renders the playhead slider initialized to the window start', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    expect(slider).toHaveAttribute('aria-valuemin', '0');
    expect(slider).toHaveAttribute('aria-valuemax', '8');
    expect(slider).toHaveAttribute('aria-valuenow', '0');
  });

  it('shows the operator total at the current bin in the flow bar', () => {
    renderOverlay(RESPONSE);
    // Bin 0: queueing/memory = 1.
    expect(screen.getByTestId('node-flow-bar')).toHaveTextContent('1.0');
  });

  it('advances one bin per ArrowRight and updates the flow bar', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    expect(slider).toHaveAttribute('aria-valuenow', '2');
    // Bin 1: queueing 2 + computing 1 = 3.
    expect(screen.getByTestId('node-flow-bar')).toHaveTextContent('3.0');
  });

  it('jumps to the window end on End and keeps constant height with no data', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'End' });
    expect(slider).toHaveAttribute('aria-valuenow', '8');
    // Last bin is all-zero for op-1: label collapses to a non-breaking space.
    const bar = screen.getByTestId('node-flow-bar');
    expect(bar).not.toHaveTextContent('1.0');
    expect(bar.textContent).toContain('\u00A0');
  });
});
