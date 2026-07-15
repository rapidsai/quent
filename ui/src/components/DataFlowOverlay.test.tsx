// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { Provider } from 'jotai';
import { render, screen, fireEvent } from '@testing-library/react';
import { useDataFlowSync } from '@quent/hooks';
import { DagPlayhead, NodeFlowBar } from '@quent/components';
import type { DataFlowTimelineResponse, EntityRef, QueryBundle } from '@quent/utils';

// 4 bins of 2s over [0, 8): op-1 task totals per bin are [1, 3, 5, 0] and
// byte totals are [0, 1500000, 0, 0].
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
      measures: [
        { name: 'tasks', display_name: 'Tasks', quantity: 'unit', kind: 'Occupancy' },
        { name: 'bytes', display_name: 'Bytes', quantity: 'capacity_bytes', kind: 'Occupancy' },
      ],
    },
    operators: {
      'op-1': {
        values: {
          tasks: {
            queueing: { memory: [1, 2, 0, 0] },
            computing: { memory: [0, 1, 3, 0], filesystem: [0, 0, 2, 0] },
          },
          bytes: {
            computing: { memory: [0, 1500000, 0, 0] },
          },
        },
      },
    },
  },
};

// Same op-1 as RESPONSE plus a huge op-2: the window max (1000) squeezes
// op-1's segments below label width (1/1000 of the ~168px track).
const NARROW_RESPONSE: DataFlowTimelineResponse = {
  Binned: {
    ...RESPONSE.Binned,
    operators: {
      ...RESPONSE.Binned.operators,
      'op-2': {
        values: {
          tasks: {
            queueing: { memory: [0, 0, 0, 1000] },
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
    capacity_bytes: {
      symbol: 'B',
      singular: 'byte',
      plural: 'bytes',
      occupancy_prefix: 'Iec',
      rate_prefix: 'Si',
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

function segmentLabels(): string[] {
  return screen.queryAllByTestId('flow-segment-label').map(el => el.textContent ?? '');
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

  it('shows totals for every measure with data at the current bin', () => {
    renderOverlay(RESPONSE);
    // Bin 0: tasks 1, bytes 0 — the zero measure is omitted.
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('1');
  });

  it('shows in-segment labels when segments are wide enough', () => {
    renderOverlay(RESPONSE);
    // Bin 0: single queueing segment, 1/5 of ~168px = ~34px — fits "1".
    expect(segmentLabels()).toEqual(['1']);
  });

  it('advances one bin per ArrowRight and joins both measure totals', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    expect(slider).toHaveAttribute('aria-valuenow', '2');
    // Bin 1: tasks queueing 2 + computing 1 = 3; bytes 1500000 -> "1.4MiB".
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('3 | 1.4MiB');
    // Both segments are wide enough (2/5 and 1/5 of ~168px).
    expect(segmentLabels()).toEqual(['2', '1']);
  });

  it('hides in-segment labels when segments are too narrow', () => {
    renderOverlay(NARROW_RESPONSE);
    // Bin 0: op-1 total is 1 against a window max of 1000 — the segment is
    // a fraction of a pixel, so no label fits, but the totals line remains.
    expect(segmentLabels()).toEqual([]);
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('1');
  });

  it('jumps to the window end on End and keeps constant height with no data', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'End' });
    expect(slider).toHaveAttribute('aria-valuenow', '8');
    // Last bin is all-zero for op-1: labels collapse to a non-breaking space.
    const bar = screen.getByTestId('node-flow-bar');
    expect(segmentLabels()).toEqual([]);
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('\u00A0');
    expect(bar).toBeInTheDocument();
  });
});
