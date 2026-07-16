// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useMemo } from 'react';
import { Panel } from '@xyflow/react';
import {
  useNodeColoringValue,
  useEdgeColoring,
  useNodeColorPalette,
  useEdgeColorPalette,
  useSelectedColorField,
  useSelectedEdgeColorField,
  useDataFlowEnabled,
  useDataFlowMeta,
  useDataFlowFrame,
  formatDataFlowValueCompact,
  type DataFlowMeta,
} from '@quent/hooks';
import {
  cn,
  createCapacitiesColorFn,
  createDataFlowStateColorFn,
  getLegendGradientStops,
  type PaletteTheme,
} from '@quent/utils';
import { inferFieldFormatter } from '@quent/utils';
import type { NodeColoring, EdgeColoring } from '../services/query-plan/types';
import type { ContinuousPaletteName } from '@quent/utils';

const MAX_CATEGORICAL_ENTRIES = 8;

interface ContinuousLegendProps {
  field: string;
  min: number;
  max: number;
  palette: ContinuousPaletteName;
  isDark: boolean;
}

const ContinuousLegend = ({ field, min, max, palette, isDark }: ContinuousLegendProps) => {
  const fmt = inferFieldFormatter(field);
  return (
    <div className="flex flex-col gap-1">
      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wide">
        {field}
      </span>
      <div
        className="h-2 w-36 rounded-sm"
        style={{
          background: `linear-gradient(to right, ${getLegendGradientStops(palette, isDark).join(', ')})`,
        }}
      />
      <div className="flex justify-between">
        <span className="text-[10px] text-muted-foreground">{fmt(min)}</span>
        <span className="text-[10px] text-muted-foreground">{fmt(max)}</span>
      </div>
    </div>
  );
};

interface CategoricalLegendProps {
  field: string;
  categoryMap: Map<string, string>;
  /**
   * Labels rendered greyed-out (e.g. deselected data-flow tiers) — still
   * listed so the user sees what is being filtered out.
   */
  dimmedLabels?: ReadonlySet<string>;
  /**
   * Per-entry value suffix (keyed by label), rendered after the label as
   * "· <suffix>" — e.g. the tier's total at the playhead bin. Labels
   * without an entry get no suffix. Dimmed entries keep their suffix
   * (dimmed along with the rest of the entry, but not struck through).
   */
  entrySuffixes?: ReadonlyMap<string, string>;
}

const CategoricalLegend = ({
  field,
  categoryMap,
  dimmedLabels,
  entrySuffixes,
}: CategoricalLegendProps) => {
  const entries = [...categoryMap.entries()].slice(0, MAX_CATEGORICAL_ENTRIES);
  const truncated = categoryMap.size > MAX_CATEGORICAL_ENTRIES;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wide">
        {field}
      </span>
      <div className="flex flex-col gap-0.5">
        {entries.map(([label, color]) => {
          const dimmed = dimmedLabels?.has(label) ?? false;
          const suffix = entrySuffixes?.get(label);
          return (
            <div
              key={label}
              data-dimmed={dimmed || undefined}
              className={cn('flex items-center gap-1.5', dimmed && 'opacity-40')}
            >
              <span
                className="inline-block h-2.5 w-2.5 rounded-sm shrink-0"
                style={{ backgroundColor: color }}
              />
              <span
                className={cn(
                  'text-[10px] text-muted-foreground truncate max-w-[120px]',
                  dimmed && 'line-through'
                )}
              >
                {label}
              </span>
              {suffix != null && (
                <span
                  data-testid="legend-entry-total"
                  className="text-[10px] text-muted-foreground tabular-nums whitespace-nowrap"
                >
                  · {suffix}
                </span>
              )}
            </div>
          );
        })}
        {truncated && (
          <span className="text-[10px] text-muted-foreground italic">
            +{categoryMap.size - MAX_CATEGORICAL_ENTRIES} more
          </span>
        )}
      </div>
    </div>
  );
};

function NodeLegendContent({
  coloring,
  field,
  palette,
  isDark,
}: {
  coloring: NodeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
  isDark: boolean;
}) {
  if (!coloring || !field) return null;
  if (coloring.type === 'continuous') {
    return (
      <ContinuousLegend
        field={field}
        min={coloring.min}
        max={coloring.max}
        palette={palette}
        isDark={isDark}
      />
    );
  }
  return <CategoricalLegend field={field} categoryMap={coloring.categoryMap} />;
}

function EdgeLegendContent({
  coloring,
  field,
  palette,
  isDark,
}: {
  coloring: EdgeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
  isDark: boolean;
}) {
  if (!coloring || !field) return null;
  if (coloring.type === 'continuous') {
    return (
      <ContinuousLegend
        field={field}
        min={coloring.min}
        max={coloring.max}
        palette={palette}
        isDark={isDark}
      />
    );
  }
  return <CategoricalLegend field={field} categoryMap={coloring.categoryMap} />;
}

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
const DataFlowTierLegend = memo(function DataFlowTierLegend({
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

interface DAGLegendProps {
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
}

/** Panel overlay showing node/edge coloring legends within the ReactFlow canvas. */
export const DAGLegend = ({ isDark }: DAGLegendProps) => {
  const nodeColoring = useNodeColoringValue();
  const edgeColoring = useEdgeColoring();
  const [nodePalette] = useNodeColorPalette();
  const [edgePalette] = useEdgeColorPalette();
  const [nodeField] = useSelectedColorField();
  const [edgeField] = useSelectedEdgeColorField();
  const dataFlowEnabled = useDataFlowEnabled();
  const dataFlowMeta = useDataFlowMeta();
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';

  // Data-flow overlay legends: FSM states (colored like the timeline view)
  // and the server-declared dimension keys (colored like capacity series).
  const dataFlowStateLegend = useMemo(() => {
    if (!dataFlowMeta) return null;
    const colorFn = createDataFlowStateColorFn(
      dataFlowMeta.fsmType,
      dataFlowMeta.stateNames,
      paletteTheme
    );
    return new Map(dataFlowMeta.stateNames.map(state => [state, colorFn(state)]));
  }, [dataFlowMeta, paletteTheme]);

  const dataFlowDimensionLegend = useMemo(() => {
    if (!dataFlowMeta) return null;
    const keys = dataFlowMeta.decl.dimension_keys;
    const colorFn = createCapacitiesColorFn(
      keys.map(k => k.key),
      paletteTheme
    );
    return new Map(keys.map(k => [k.display_name, colorFn(k.key)]));
  }, [dataFlowMeta, paletteTheme]);

  // Deselected tiers stay listed but greyed-out, so the user can see what
  // the tier filter is currently hiding.
  const dimmedDimensionLabels = useMemo(() => {
    if (!dataFlowMeta) return undefined;
    return new Set(
      dataFlowMeta.decl.dimension_keys
        .filter(k => !dataFlowMeta.dimensionSelection.has(k.key))
        .map(k => k.display_name)
    );
  }, [dataFlowMeta]);

  const hasNode = !!nodeColoring && !!nodeField;
  const hasEdge = !!edgeColoring && !!edgeField;
  const hasDataFlow =
    dataFlowEnabled && !!dataFlowMeta && !!dataFlowStateLegend && !!dataFlowDimensionLegend;

  if (!hasNode && !hasEdge && !hasDataFlow) return null;

  return (
    <Panel position="bottom-left">
      <div className="flex flex-col gap-2.5 rounded-md border bg-card/90 backdrop-blur-sm px-3 py-2.5 shadow-md text-card-foreground">
        <NodeLegendContent
          coloring={nodeColoring}
          field={nodeField}
          palette={nodePalette}
          isDark={isDark}
        />
        {hasNode && hasEdge && <div className="border-t border-border" />}
        <EdgeLegendContent
          coloring={edgeColoring}
          field={edgeField}
          palette={edgePalette}
          isDark={isDark}
        />
        {(hasNode || hasEdge) && hasDataFlow && <div className="border-t border-border" />}
        {hasDataFlow && (
          <>
            <CategoricalLegend
              field={dataFlowMeta.decl.entity_type_name}
              categoryMap={dataFlowStateLegend}
            />
            <DataFlowTierLegend
              meta={dataFlowMeta}
              categoryMap={dataFlowDimensionLegend}
              dimmedLabels={dimmedDimensionLabels}
            />
          </>
        )}
      </div>
    </Panel>
  );
};
