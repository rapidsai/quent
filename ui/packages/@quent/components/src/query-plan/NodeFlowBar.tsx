// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useMemo } from 'react';
import {
  createCapacitiesColorFn,
  createFsmTypeColorFn,
  type FsmTypeDecl,
  type PaletteTheme,
} from '@quent/utils';
import { useDataFlowFrame, useDataFlowMeta, formatDataFlowValue } from '@quent/hooks';

const BAR_TRANSITION = 'width 120ms linear';

/** State colors keyed on the FSM type declaration — matches the timeline view. */
function fsmTypesMapOf(fsmType: FsmTypeDecl | null): { [key in string]?: FsmTypeDecl } {
  return fsmType ? { [fsmType.name]: fsmType } : {};
}

/**
 * Per-node data-flow overlay: a stacked state bar over a thin dimension bar,
 * plus a tiny total label. CRITICAL PERF: this is the only node-level
 * subscriber to the frame atom — a scrub tick re-renders these tiny bars,
 * not the full `QueryPlanNode`s.
 *
 * Constant height whether or not the operator has data at the current bin,
 * so scrubbing never causes layout churn.
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

    return (
      <div className="mt-1.5 w-full" data-testid="node-flow-bar">
        <div className="h-[6px] w-full overflow-hidden rounded-sm bg-muted/40">
          <div className="flex h-full" style={{ width: filledWidth, transition: BAR_TRANSITION }}>
            {hasData &&
              meta.stateNames.map((state, stateIndex) => {
                const value = operatorFrame.byState[stateIndex] ?? 0;
                if (value <= 0) return null;
                return (
                  <div
                    key={state}
                    style={{ flexGrow: value, backgroundColor: stateColor(state) }}
                  />
                );
              })}
          </div>
        </div>
        <div className="mt-[2px] h-[3px] w-full overflow-hidden rounded-sm bg-muted/40">
          <div className="flex h-full" style={{ width: filledWidth, transition: BAR_TRANSITION }}>
            {hasData &&
              meta.decl.dimension_keys.map((dimension, dimensionIndex) => {
                const value = operatorFrame.byDimension[dimensionIndex] ?? 0;
                if (value <= 0) return null;
                return (
                  <div
                    key={dimension.key}
                    style={{ flexGrow: value, backgroundColor: dimensionColor(dimension.key) }}
                  />
                );
              })}
          </div>
        </div>
        <div className="text-right text-[9px] leading-3 text-muted-foreground tabular-nums">
          {hasData ? formatDataFlowValue(total, frame.measure, meta) : '\u00A0'}
        </div>
      </div>
    );
  }
);

NodeFlowBar.displayName = 'NodeFlowBar';
