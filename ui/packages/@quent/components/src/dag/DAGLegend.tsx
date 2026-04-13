// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Panel } from '@xyflow/react';
import {
  useNodeColoringValue,
  useEdgeColoring,
  useNodeColorPalette,
  useEdgeColorPalette,
  useSelectedColorField,
  useSelectedEdgeColorField,
} from '@quent/hooks';
import { getLegendGradientStops } from '@quent/utils';
import { inferFieldFormatter } from '../services/query-plan/dagFieldProcessing';
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

  const hasNode = !!nodeColoring && !!nodeField;
  const hasEdge = !!edgeColoring && !!edgeField;

  if (!hasNode && !hasEdge) return null;

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
      </div>
    </Panel>
  );
};
