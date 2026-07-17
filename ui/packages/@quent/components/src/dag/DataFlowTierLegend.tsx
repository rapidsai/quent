// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useMemo } from 'react';
import { useDataFlowFrame, formatDataFlowValueCompact, type DataFlowMeta } from '@quent/hooks';
import { CategoricalLegend } from './DAGLegend';

interface DataFlowTierLegendProps {
  meta: DataFlowMeta;
  /** Tier display name -> swatch color (see `dataFlowDimensionLegend`). */
  categoryMap: Map<string, string>;
  dimmedLabels?: ReadonlySet<string>;
}

/**
 * Dimension (tier) group of the data-flow legend, annotating each tier with
 * its TOTAL at the playhead's bin — summed over all operators and states —
 * in the current flow measure (e.g. "GPU-0 · 12.4GiB" = total memory held
 * by that tier at this point in time). Zero totals get no suffix (matching
 * the in-bar labels, which hide zeros); deselected tiers keep their totals,
 * dimmed with the rest of the entry.
 *
 * Isolated in a memoized leaf so that only this subtree subscribes to the
 * per-scrub-tick frame — the rest of the legend re-renders only when the
 * meta (response/tier selection) changes.
 */
export const DataFlowTierLegend = memo(function DataFlowTierLegend({
  meta,
  categoryMap,
  dimmedLabels,
}: DataFlowTierLegendProps) {
  const frame = useDataFlowFrame();
  const entrySuffixes = useMemo(() => {
    if (!frame) return undefined;
    const totals = frame.dimensionTotalsByMeasure[frame.measure];
    if (!totals) return undefined;
    const suffixes = new Map<string, string>();
    meta.decl.dimension_keys.forEach((k, index) => {
      const total = totals[index] ?? 0;
      if (total > 0) {
        suffixes.set(k.display_name, formatDataFlowValueCompact(total, frame.measure, meta));
      }
    });
    return suffixes;
  }, [frame, meta]);
  return (
    <CategoricalLegend
      field={meta.decl.dimension_name}
      categoryMap={categoryMap}
      dimmedLabels={dimmedLabels}
      entrySuffixes={entrySuffixes}
    />
  );
});
