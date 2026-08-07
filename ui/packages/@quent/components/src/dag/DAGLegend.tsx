// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
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
} from '@quent/hooks';
import {
  cn,
  createCapacitiesColorFn,
  createDataFlowStateColorFn,
  getLegendGradientStops,
  type PaletteTheme,
} from '@quent/utils';
import { inferFieldFormatter, formatQuantity, type QuantitySpec } from '@quent/utils';
import { DataFlowTierLegend } from './DataFlowTierLegend';
import type { NodeColoring, EdgeColoring } from '../services/query-plan/types';
import type { ContinuousPaletteName } from '@quent/utils';

const MAX_CATEGORICAL_ENTRIES = 8;

interface ContinuousLegendProps {
  field: string;
  min: number;
  max: number;
  palette: ContinuousPaletteName;
  isDark: boolean;
  formatValue?: (v: number) => string;
}

const ContinuousLegend = ({
  field,
  min,
  max,
  palette,
  isDark,
  formatValue,
}: ContinuousLegendProps) => {
  const fmt = formatValue ?? inferFieldFormatter(field);
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

/** Shared category-swatch legend group (also used by `DataFlowTierLegend`). */
export const CategoricalLegend = ({
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

function resolveFormatter(
  field: string,
  statQuantitySpecs: Record<string, QuantitySpec>
): ((v: number) => string) | undefined {
  const spec = statQuantitySpecs[field];
  return spec ? (v: number) => formatQuantity(v, spec, 'Occupancy') : undefined;
}

function NodeLegendContent({
  coloring,
  field,
  palette,
  isDark,
  statQuantitySpecs,
}: {
  coloring: NodeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
  isDark: boolean;
  statQuantitySpecs: Record<string, QuantitySpec>;
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
        formatValue={resolveFormatter(field, statQuantitySpecs)}
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
  statQuantitySpecs,
}: {
  coloring: EdgeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
  isDark: boolean;
  statQuantitySpecs: Record<string, QuantitySpec>;
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
        formatValue={resolveFormatter(field, statQuantitySpecs)}
      />
    );
  }
  return <CategoricalLegend field={field} categoryMap={coloring.categoryMap} />;
}

interface DAGLegendProps {
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
  /** Pre-resolved stat-key → QuantitySpec for quantity-aware legend formatting. */
  statQuantitySpecs?: Record<string, QuantitySpec>;
}

/** Panel overlay showing node/edge coloring legends within the ReactFlow canvas. */
export const DAGLegend = ({ isDark, statQuantitySpecs = {} }: DAGLegendProps) => {
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
          statQuantitySpecs={statQuantitySpecs}
        />
        {hasNode && hasEdge && <div className="border-t border-border" />}
        <EdgeLegendContent
          coloring={edgeColoring}
          field={edgeField}
          palette={edgePalette}
          isDark={isDark}
          statQuantitySpecs={statQuantitySpecs}
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
