// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useMemo } from 'react';
import {
  createCapacitiesColorFn,
  createFsmTypeColorFn,
  isLightColor,
  type FsmTypeDecl,
  type PaletteTheme,
} from '@quent/utils';
import {
  useDataFlowFrame,
  useDataFlowMeta,
  fitDataFlowSegmentLabel,
  formatDataFlowValueCompact,
} from '@quent/hooks';
import { NODE_LAYOUT_WIDTH } from '../dag/layout';

const BAR_TRANSITION = 'width 120ms linear';

/**
 * Usable track width in pixels: the node is laid out at a fixed
 * {@link NODE_LAYOUT_WIDTH} with `px-4` (16px) padding on each side. Used to
 * width-gate in-segment labels without DOM measurement.
 */
const FLOW_TRACK_PX = NODE_LAYOUT_WIDTH - 32;

/** State colors keyed on the FSM type declaration — matches the timeline view. */
function fsmTypesMapOf(fsmType: FsmTypeDecl | null): { [key in string]?: FsmTypeDecl } {
  return fsmType ? { [fsmType.name]: fsmType } : {};
}

/** Width-gated value label centered inside an overflow-hidden segment. */
const SegmentValueLabel = ({
  label,
  segmentColor,
  testId,
}: {
  label: string;
  segmentColor: string;
  testId: string;
}) => (
  <span
    data-testid={testId}
    className="absolute inset-0 flex items-center justify-center text-[8px] leading-none font-medium tabular-nums whitespace-nowrap"
    style={
      isLightColor(segmentColor)
        ? { color: 'rgba(0, 0, 0, 0.78)' }
        : { color: '#ffffff', textShadow: '0 0 2px rgba(0, 0, 0, 0.45)' }
    }
  >
    {label}
  </span>
);

/**
 * Per-node data-flow overlay: a stacked state bar over a stacked
 * dimension/tier bar — both 12px with width-gated in-segment value labels —
 * plus a tiny totals label covering every declared measure. Widths are
 * driven by the bar measure; in-segment labels by `frame.labelMeasure`
 * (which follows the bar measure unless the user picked an independent
 * one). Only the SELECTED tiers contribute (unselected dimension columns
 * are zero in the frame). CRITICAL PERF: this is the only node-level
 * subscriber to the frame atom — a scrub tick re-renders these tiny bars,
 * not the full `QueryPlanNode`s.
 *
 * Constant height whether or not the operator has data at the current bin
 * (the empty tracks are the placeholders), so scrubbing never causes layout
 * churn. Labels are absolutely positioned inside overflow-hidden segments,
 * so their appearance never shifts layout. The two bars stay visually
 * distinct: FSM state colors on top, capacity/tier colors below, separated
 * by a 2px gap.
 */
export const NodeFlowBar = memo(
  ({ operatorId, isDark }: { operatorId: string; isDark: boolean }) => {
    const meta = useDataFlowMeta();
    const frame = useDataFlowFrame();
    const theme: PaletteTheme = isDark ? 'dark' : 'light';

    const fsmType = meta?.fsmType ?? null;
    const stateColor = useMemo(
      () => createFsmTypeColorFn(fsmTypesMapOf(fsmType), theme),
      [fsmType, theme]
    );
    const dimensionKeys = meta?.decl.dimension_keys;
    const dimensionColor = useMemo(
      () =>
        createCapacitiesColorFn(
          (dimensionKeys ?? []).map(k => k.key),
          theme
        ),
      [dimensionKeys, theme]
    );

    if (!meta || !frame) return null;

    const operatorFrame = frame.perOperator.get(operatorId);
    const total = operatorFrame?.total ?? 0;
    const hasData = operatorFrame != null && total > 0 && frame.maxTotal > 0;
    // Stable scale while scrubbing: filled width is relative to the max
    // operator total across ALL bins of the window (frame.maxTotal).
    const filledWidth = hasData ? `max(2px, ${(total / frame.maxTotal) * 100}%)` : '0px';

    // One compact total per declared measure with data at this bin, in
    // declaration order — e.g. "3.2 | 45MB" (count | bytes). A pipe, not a
    // middot: "121 · 5GB" reads like the decimal "121.5GB".
    const operatorTotals = frame.totalsByMeasure.get(operatorId);
    const totalsLabel = operatorTotals
      ? meta.decl.measures
          .filter(m => (operatorTotals[m.name] ?? 0) > 0)
          .map(m => formatDataFlowValueCompact(operatorTotals[m.name]!, m.name, meta))
          .join(' | ')
      : '';

    return (
      <div className="mt-1.5 w-full" data-testid="node-flow-bar">
        <div className="h-[12px] w-full overflow-hidden rounded-sm bg-muted/40">
          <div className="flex h-full" style={{ width: filledWidth, transition: BAR_TRANSITION }}>
            {hasData &&
              meta.stateNames.map((state, stateIndex) => {
                const value = operatorFrame.byState[stateIndex] ?? 0;
                if (value <= 0) return null;
                const color = stateColor(state);
                const label = fitDataFlowSegmentLabel(
                  value,
                  frame.maxTotal,
                  frame.measure,
                  meta,
                  FLOW_TRACK_PX,
                  {
                    value: operatorFrame.labelByState[stateIndex] ?? 0,
                    measure: frame.labelMeasure,
                  }
                );
                return (
                  <div
                    key={state}
                    className="relative overflow-hidden"
                    style={{ flexGrow: value, backgroundColor: color }}
                  >
                    {label != null && (
                      <SegmentValueLabel
                        label={label}
                        segmentColor={color}
                        testId="flow-segment-label"
                      />
                    )}
                  </div>
                );
              })}
          </div>
        </div>
        <div className="mt-[2px] h-[12px] w-full overflow-hidden rounded-sm bg-muted/40">
          <div className="flex h-full" style={{ width: filledWidth, transition: BAR_TRANSITION }}>
            {hasData &&
              meta.decl.dimension_keys.map((dimension, dimensionIndex) => {
                const value = operatorFrame.byDimension[dimensionIndex] ?? 0;
                if (value <= 0) return null;
                const color = dimensionColor(dimension.key);
                const label = fitDataFlowSegmentLabel(
                  value,
                  frame.maxTotal,
                  frame.measure,
                  meta,
                  FLOW_TRACK_PX,
                  {
                    value: operatorFrame.labelByDimension[dimensionIndex] ?? 0,
                    measure: frame.labelMeasure,
                  }
                );
                return (
                  <div
                    key={dimension.key}
                    className="relative overflow-hidden"
                    style={{ flexGrow: value, backgroundColor: color }}
                  >
                    {label != null && (
                      <SegmentValueLabel
                        label={label}
                        segmentColor={color}
                        testId="flow-tier-label"
                      />
                    )}
                  </div>
                );
              })}
          </div>
        </div>
        <div
          className="text-right text-[9px] leading-3 text-muted-foreground tabular-nums truncate"
          data-testid="flow-bar-totals"
        >
          {totalsLabel !== '' ? totalsLabel : '\u00A0'}
        </div>
      </div>
    );
  }
);

NodeFlowBar.displayName = 'NodeFlowBar';
