// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Panel } from '@xyflow/react';
import { useAtomValue } from 'jotai';
import {
  nodeColoringAtom,
  edgeColoringAtom,
  nodeColorPaletteAtom,
  edgeColorPaletteAtom,
  selectedColorField,
  selectedEdgeColorFieldAtom,
} from '@/atoms/dagControls';
import { getLegendGradientStops } from '@quent/utils';
import { inferFieldFormatter } from '@/services/query-plan/dagFieldProcessing';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import type { NodeColoring, EdgeColoring } from '@/services/query-plan/types';
import type { ContinuousPaletteName } from '@quent/utils';

const MAX_CATEGORICAL_ENTRIES = 8;

interface ContinuousLegendProps {
  field: string;
  min: number;
  max: number;
  palette: ContinuousPaletteName;
}

const ContinuousLegend = ({ field, min, max, palette }: ContinuousLegendProps) => {
  const fmt = inferFieldFormatter(field);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wide">
        {field}
      </span>
      <div
        className="h-2 w-36 rounded-sm"
        style={{
          background: `linear-gradient(to right, ${getLegendGradientStops(palette, isDarkMode).join(', ')})`,
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
}

const CategoricalLegend = ({ field, categoryMap }: CategoricalLegendProps) => {
  const entries = [...categoryMap.entries()].slice(0, MAX_CATEGORICAL_ENTRIES);
  const truncated = categoryMap.size > MAX_CATEGORICAL_ENTRIES;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wide">
        {field}
      </span>
      <div className="flex flex-col gap-0.5">
        {entries.map(([label, color]) => (
          <div key={label} className="flex items-center gap-1.5">
            <span
              className="inline-block h-2.5 w-2.5 rounded-sm shrink-0"
              style={{ backgroundColor: color }}
            />
            <span className="text-[10px] text-muted-foreground truncate max-w-[120px]">
              {label}
            </span>
          </div>
        ))}
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
}: {
  coloring: NodeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
}) {
  if (!coloring || !field) return null;
  if (coloring.type === 'continuous') {
    return (
      <ContinuousLegend field={field} min={coloring.min} max={coloring.max} palette={palette} />
    );
  }
  return <CategoricalLegend field={field} categoryMap={coloring.categoryMap} />;
}

function EdgeLegendContent({
  coloring,
  field,
  palette,
}: {
  coloring: EdgeColoring;
  field: string | null;
  palette: ContinuousPaletteName;
}) {
  if (!coloring || !field) return null;
  if (coloring.type === 'continuous') {
    return (
      <ContinuousLegend field={field} min={coloring.min} max={coloring.max} palette={palette} />
    );
  }
  return <CategoricalLegend field={field} categoryMap={coloring.categoryMap} />;
}

export const DAGLegend = () => {
  const nodeColoring = useAtomValue(nodeColoringAtom);
  const edgeColoring = useAtomValue(edgeColoringAtom);
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const edgePalette = useAtomValue(edgeColorPaletteAtom);
  const nodeField = useAtomValue(selectedColorField);
  const edgeField = useAtomValue(selectedEdgeColorFieldAtom);

  const hasNode = !!nodeColoring && !!nodeField;
  const hasEdge = !!edgeColoring && !!edgeField;

  if (!hasNode && !hasEdge) return null;

  return (
    <Panel position="bottom-left">
      <div className="flex flex-col gap-2.5 rounded-md border bg-card/90 backdrop-blur-sm px-3 py-2.5 shadow-md text-card-foreground">
        <NodeLegendContent coloring={nodeColoring} field={nodeField} palette={nodePalette} />
        {hasNode && hasEdge && <div className="border-t border-border" />}
        <EdgeLegendContent coloring={edgeColoring} field={edgeField} palette={edgePalette} />
      </div>
    </Panel>
  );
};
