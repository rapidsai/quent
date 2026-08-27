// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useMemo } from 'react';
import {
  createCapacitiesColorFn,
  createDataFlowStateColorFn,
  type PaletteTheme,
} from '@quent/utils';
import {
  useDataFlowFrame,
  useDataFlowMeta,
  fitDataFlowSegmentLabel,
  formatDataFlowValueCompact,
} from '@quent/hooks';
import {
  NODE_LAYOUT_WIDTH,
  FLOW_BAR_TOP_MARGIN,
  FLOW_BAR_TRACK_HEIGHT,
  FLOW_BAR_TRACK_GAP,
  FLOW_BAR_LABEL_HEIGHT,
} from '../dag/layout';
import { SegmentedBar } from '../segmented-bar/SegmentedBar';

const BAR_TRANSITION = 'width 120ms linear';

/**
 * Usable track width in pixels: the node is laid out at a fixed
 * {@link NODE_LAYOUT_WIDTH} with `px-4` (16px) padding on each side. Used to
 * width-gate in-segment labels without DOM measurement.
 */
const FLOW_TRACK_PX = NODE_LAYOUT_WIDTH - 32;

/** State colors keyed on the FSM type declaration — matches the timeline view. */
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
    const stateNames = meta?.stateNames;
    const stateColor = useMemo(
      () => createDataFlowStateColorFn(fsmType, stateNames ?? [], theme),
      [fsmType, stateNames, theme]
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

    if (!meta || !frame) {
      return null;
    }

    const operatorFrame = frame.perOperator.get(operatorId);
    const total = operatorFrame?.total ?? 0;
    const hasData = operatorFrame != null && total > 0 && frame.maxTotal > 0;

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
    const stateSegments = hasData
      ? meta.stateNames.flatMap((state, stateIndex) => {
          const value = operatorFrame.byState[stateIndex] ?? 0;
          if (value <= 0) {
            return [];
          }
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
          return [{ id: state, value, color, label: label ?? undefined }];
        })
      : [];
    const dimensionSegments = hasData
      ? meta.decl.dimension_keys.flatMap((dimension, dimensionIndex) => {
          const value = operatorFrame.byDimension[dimensionIndex] ?? 0;
          if (value <= 0) {
            return [];
          }
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
          return [{ id: dimension.key, value, color, label: label ?? undefined }];
        })
      : [];

    return (
      <div
        className="w-full"
        style={{ marginTop: FLOW_BAR_TOP_MARGIN }}
        data-testid="node-flow-bar"
      >
        <SegmentedBar
          segments={stateSegments}
          fillValue={total}
          maxValue={frame.maxTotal}
          height={FLOW_BAR_TRACK_HEIGHT}
          minimumFillPx={2}
          showTooltips={false}
          transition={BAR_TRANSITION}
          trackClassName={hasData ? 'bg-muted/40' : 'bg-transparent'}
          labelTestId="flow-segment-label"
        />
        <SegmentedBar
          segments={dimensionSegments}
          fillValue={total}
          maxValue={frame.maxTotal}
          height={FLOW_BAR_TRACK_HEIGHT}
          minimumFillPx={2}
          showTooltips={false}
          transition={BAR_TRANSITION}
          style={{ marginTop: FLOW_BAR_TRACK_GAP }}
          trackClassName={hasData ? 'bg-muted/40' : 'bg-transparent'}
          labelTestId="flow-tier-label"
        />
        <div
          className="text-right text-[9px] text-muted-foreground tabular-nums truncate"
          style={{ lineHeight: `${FLOW_BAR_LABEL_HEIGHT}px` }}
          data-testid="flow-bar-totals"
        >
          {totalsLabel !== '' ? totalsLabel : '\u00A0'}
        </div>
      </div>
    );
  }
);

NodeFlowBar.displayName = 'NodeFlowBar';
