// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, afterEach } from 'vitest';
import { useEffect } from 'react';
import { Provider } from 'jotai';
import { ReactFlowProvider } from '@xyflow/react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import {
  useDataFlowSync,
  useSetDataFlowEnabled,
  useSetDataFlowLabelMeasure,
  useSetDataFlowSelectedDimensions,
  useSetSelectedNodeData,
} from '@quent/hooks';
import {
  DagPlayhead,
  DAGLegend,
  DAGNodeInfoPanel,
  NodeFlowBar,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
} from '@quent/components';
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
      default_measure: null,
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

// One dominant queueing segment at bin 0 (4/5 of the bar ≈ 134px) so a byte
// label ("1.4MiB", 40px) fits inside it — exercises the label-measure toggle.
const LABEL_RESPONSE: DataFlowTimelineResponse = {
  Binned: {
    ...RESPONSE.Binned,
    operators: {
      'op-1': {
        values: {
          tasks: {
            queueing: { memory: [4, 0, 0, 0] },
            computing: { memory: [1, 0, 0, 0] },
          },
          bytes: {
            queueing: { memory: [1500000, 0, 0, 0] },
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

interface HarnessProps {
  response: DataFlowTimelineResponse;
  /** Whether the data-flow overlay is enabled (defaults to true). */
  enabled?: boolean;
  /** In-segment label measure (null = follow the bar measure). */
  labelMeasure?: string | null;
  /** Tier selection (null = all declared dimension keys). */
  selectedDimensions?: ReadonlySet<string> | null;
  children?: React.ReactNode;
}

function Harness({
  response,
  enabled = true,
  labelMeasure = null,
  selectedDimensions = null,
  children,
}: HarnessProps) {
  useDataFlowSync({ response, queryBundle: QUERY_BUNDLE });
  const setEnabled = useSetDataFlowEnabled();
  const setLabelMeasure = useSetDataFlowLabelMeasure();
  const setSelectedDimensions = useSetDataFlowSelectedDimensions();
  useEffect(() => {
    setEnabled(enabled);
  }, [enabled, setEnabled]);
  useEffect(() => {
    setLabelMeasure(labelMeasure);
  }, [labelMeasure, setLabelMeasure]);
  useEffect(() => {
    setSelectedDimensions(selectedDimensions);
  }, [selectedDimensions, setSelectedDimensions]);
  return (
    children ?? (
      <>
        <DagPlayhead startTimeUnixNs={BigInt(0)} />
        <NodeFlowBar operatorId="op-1" isDark={false} />
      </>
    )
  );
}

function renderOverlay(props: DataFlowTimelineResponse | HarnessProps) {
  const harnessProps: HarnessProps =
    typeof props === 'object' && 'response' in props ? props : { response: props };
  return render(
    <Provider>
      <Harness {...harnessProps} />
    </Provider>
  );
}

function segmentLabels(): string[] {
  return screen.queryAllByTestId('flow-segment-label').map(el => el.textContent ?? '');
}

function tierLabels(): string[] {
  return screen.queryAllByTestId('flow-tier-label').map(el => el.textContent ?? '');
}

/** flex-grow values (segment width weights) of the state bar's segments. */
function stateSegmentWidths(): string[] {
  const bar = screen.getByTestId('node-flow-bar');
  const fill = (bar.children[0] as HTMLElement).children[0] as HTMLElement;
  return [...fill.children].map(el => (el as HTMLElement).style.flexGrow);
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
    expect(tierLabels()).toEqual([]);
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('\u00A0');
    expect(bar).toBeInTheDocument();
  });

  it('renders both bars at the same labeled height (constant node height)', () => {
    renderOverlay(RESPONSE);
    const bar = screen.getByTestId('node-flow-bar');
    const stateTrack = bar.children[0] as HTMLElement;
    const tierTrack = bar.children[1] as HTMLElement;
    expect(stateTrack.className).toContain('h-[12px]');
    expect(tierTrack.className).toContain('h-[12px]');
    expect(tierTrack.className).toContain('mt-[2px]');
  });
});

describe('playback while the overlay is disabled', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('stops the play interval and hides the synced pointer when disabled', () => {
    vi.useFakeTimers();
    // Fake timeline chart: receives the showTip/hideTip actions that
    // broadcastSyncedPointer/hideSyncedPointer dispatch to registered charts.
    const dispatchAction = vi.fn();
    const fakeChart = {
      convertToPixel: () => 42,
      getHeight: () => 100,
      dispatchAction,
      getZr: () => ({ on: vi.fn(), off: vi.fn() }),
    } as unknown as Parameters<typeof registerAxisPointerSync>[0];
    registerAxisPointerSync(fakeChart);
    try {
      const { rerender } = renderOverlay(RESPONSE);
      fireEvent.click(screen.getByRole('button', { name: 'Play data flow' }));
      act(() => {
        vi.advanceTimersByTime(100);
      });
      // One tick advanced one bin (2s) and broadcast the synced crosshair.
      expect(screen.getByRole('slider')).toHaveAttribute('aria-valuenow', '2');
      expect(dispatchAction).toHaveBeenCalledWith(expect.objectContaining({ type: 'showTip' }));
      dispatchAction.mockClear();

      // Disable the overlay mid-playback: the component renders null but
      // stays mounted, so the interval must stop and the crosshair hide.
      rerender(
        <Provider>
          <Harness response={RESPONSE} enabled={false} />
        </Provider>
      );
      expect(screen.queryByTestId('dag-playhead')).not.toBeInTheDocument();
      expect(dispatchAction).toHaveBeenCalledWith({ type: 'hideTip' });
      dispatchAction.mockClear();

      // No further ticks: nothing is broadcast while disabled...
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      expect(dispatchAction).not.toHaveBeenCalled();

      // ...and re-enabling shows a paused playhead that did not advance.
      rerender(
        <Provider>
          <Harness response={RESPONSE} enabled />
        </Provider>
      );
      expect(screen.getByRole('slider')).toHaveAttribute('aria-valuenow', '2');
      expect(screen.getByRole('button', { name: 'Play data flow' })).toBeInTheDocument();
    } finally {
      unregisterAxisPointerSync(fakeChart);
    }
  });
});

describe('analyzer-declared default measure', () => {
  // Same data as RESPONSE, but the analyzer declares bytes as the default.
  const BYTES_DEFAULT_RESPONSE: DataFlowTimelineResponse = {
    Binned: {
      ...RESPONSE.Binned,
      decl: { ...RESPONSE.Binned.decl, default_measure: 'bytes' },
    },
  };

  it('starts the flow bars on the declared default measure', () => {
    renderOverlay(BYTES_DEFAULT_RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Bin 1 under bytes: one full-width 1500000-byte segment. Under tasks
    // (the first declared measure) the labels would read ['2', '1'].
    expect(segmentLabels()).toEqual(['1.4MiB']);
  });
});

describe('segment-label measure toggle', () => {
  it('switches in-segment texts to the label measure without changing widths', () => {
    const { rerender } = renderOverlay(LABEL_RESPONSE);
    // Bin 0, labels follow the bar measure (tasks): queueing 4, computing 1.
    expect(segmentLabels()).toEqual(['4', '1']);
    const widthsBefore = stateSegmentWidths();
    expect(widthsBefore).toEqual(['4', '1']);

    rerender(
      <Provider>
        <Harness response={LABEL_RESPONSE} labelMeasure="bytes" />
      </Provider>
    );
    // Texts now come from bytes: queueing 1500000 -> "1.4MiB"; computing has
    // zero bytes, so its label disappears instead of showing a stray "0".
    expect(segmentLabels()).toEqual(['1.4MiB']);
    // Segment widths still follow the bar measure (tasks).
    expect(stateSegmentWidths()).toEqual(widthsBefore);
  });

  it('labels the tier bar with the label measure too', () => {
    renderOverlay({ response: LABEL_RESPONSE, labelMeasure: 'bytes' });
    // Single memory tier holding all 1500000 bytes at bin 0.
    expect(tierLabels()).toEqual(['1.4MiB']);
  });
});

describe('tier bar labels', () => {
  it('shows width-gated per-tier totals inside the tier bar', () => {
    renderOverlay(RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Bin 2: memory 3 (~101px) and filesystem 2 (~67px) both fit.
    expect(tierLabels()).toEqual(['3', '2']);
  });

  it('hides tier labels when segments are too narrow', () => {
    renderOverlay(NARROW_RESPONSE);
    // op-1's bar is 1/1000 of the track \u2014 nothing fits in either bar.
    expect(tierLabels()).toEqual([]);
    expect(segmentLabels()).toEqual([]);
  });
});

describe('tier (dimension) selection', () => {
  it('recomputes widths, labels, totals and windowMax over the selection', () => {
    const { rerender } = renderOverlay(NARROW_RESPONSE);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Bin 2, all tiers: op-2's 1000 (memory, bin 3) dominates the window
    // max, so op-1's total of 5 is sub-pixel \u2014 no labels anywhere.
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('5');
    expect(segmentLabels()).toEqual([]);
    expect(tierLabels()).toEqual([]);

    rerender(
      <Provider>
        <Harness response={NARROW_RESPONSE} selectedDimensions={new Set(['filesystem'])} />
      </Provider>
    );
    // Filesystem only: op-2 vanishes from the window max (its data lives in
    // memory), which becomes op-1's filesystem peak of 2 \u2014 the bar now fills
    // the whole track, so the labels fit again (they could not at 1/1000).
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('2');
    expect(segmentLabels()).toEqual(['2']);
    expect(tierLabels()).toEqual(['2']);
  });

  it('treats an all-unknown (stale) selection as all tiers', () => {
    renderOverlay({ response: RESPONSE, selectedDimensions: new Set(['GPU-0', 'GPU-1']) });
    // Bin 0 renders exactly like the unfiltered response.
    expect(segmentLabels()).toEqual(['1']);
    expect(screen.getByTestId('flow-bar-totals').textContent).toBe('1');
  });
});

describe('DAGNodeInfoPanel matrix under tier selection', () => {
  function renderPanel(selectedDimensions: ReadonlySet<string> | null) {
    function SelectNode() {
      const setSelectedNodeData = useSetSelectedNodeData();
      useEffect(() => {
        setSelectedNodeData({
          nodeId: 'op-1',
          label: 'Op 1',
          operationType: 'scan',
          statistics: [],
        });
      }, [setSelectedNodeData]);
      return <DAGNodeInfoPanel />;
    }
    return render(
      <Provider>
        <Harness response={RESPONSE} selectedDimensions={selectedDimensions}>
          <SelectNode />
        </Harness>
      </Provider>
    );
  }

  it('shows all dimension columns when every tier is selected', () => {
    renderPanel(null);
    expect(screen.getByText('Memory')).toBeInTheDocument();
    expect(screen.getByText('Filesystem')).toBeInTheDocument();
  });

  it('hides deselected dimension columns', () => {
    renderPanel(new Set(['memory']));
    expect(screen.getByText('Memory')).toBeInTheDocument();
    expect(screen.queryByText('Filesystem')).not.toBeInTheDocument();
  });
});

describe('DAGLegend under tier selection', () => {
  function renderLegend(
    selectedDimensions: ReadonlySet<string> | null,
    response: DataFlowTimelineResponse = RESPONSE
  ) {
    return render(
      <Provider>
        <Harness response={response} selectedDimensions={selectedDimensions}>
          <DagPlayhead startTimeUnixNs={BigInt(0)} />
          <ReactFlowProvider>
            <DAGLegend isDark={false} />
          </ReactFlowProvider>
        </Harness>
      </Provider>
    );
  }

  /** Text of the "· <total>" suffix of one tier entry, `null` when absent. */
  function tierTotal(label: string): string | null {
    const entry = screen.getByText(label).parentElement!;
    return entry.querySelector('[data-testid="legend-entry-total"]')?.textContent ?? null;
  }

  it('lists every declared tier undimmed when all are selected', () => {
    renderLegend(null);
    expect(screen.getByText('Memory').closest('[data-dimmed]')).toBeNull();
    expect(screen.getByText('Filesystem').closest('[data-dimmed]')).toBeNull();
  });

  it('greys out deselected tiers instead of dropping them', () => {
    renderLegend(new Set(['memory']));
    expect(screen.getByText('Memory').closest('[data-dimmed]')).toBeNull();
    expect(screen.getByText('Filesystem').closest('[data-dimmed]')).not.toBeNull();
  });

  it('appends each tier total at the current bin; zero totals get no suffix', () => {
    renderLegend(null);
    // Bin 0 (tasks): memory 1, filesystem 0.
    expect(tierTotal('Memory')).toBe('· 1');
    expect(tierTotal('Filesystem')).toBeNull();
  });

  it('updates the tier totals when the playhead crosses bins', () => {
    renderLegend(null);
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Bin 2 (tasks): memory 3, filesystem 2.
    expect(tierTotal('Memory')).toBe('· 3');
    expect(tierTotal('Filesystem')).toBe('· 2');
  });

  it('keeps totals on dimmed (deselected) tiers', () => {
    renderLegend(new Set(['memory']));
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Filesystem is filtered out of the bars but keeps its global total.
    expect(screen.getByText('Filesystem').closest('[data-dimmed]')).not.toBeNull();
    expect(tierTotal('Filesystem')).toBe('· 2');
  });

  it('formats totals in the current flow measure via its quantity spec', () => {
    renderLegend(null, {
      Binned: {
        ...RESPONSE.Binned,
        decl: { ...RESPONSE.Binned.decl, default_measure: 'bytes' },
      },
    });
    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    // Bin 1 under the bytes measure: memory holds 1500000 bytes.
    expect(tierTotal('Memory')).toBe('· 1.4MiB');
    expect(tierTotal('Filesystem')).toBeNull();
  });
});
