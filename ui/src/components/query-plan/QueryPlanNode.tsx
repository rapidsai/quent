// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useState, useMemo, useCallback } from 'react';
import { cn } from '@/lib/utils';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import { useAtomValue, useSetAtom } from 'jotai';
import {
  selectedNodeLabelFieldAtom,
  NODE_LABEL_FIELD,
  highlightedNodeIdsAtom,
  effectiveHighlightedNodeIdsAtom,
  effectiveHoveredStatAtom,
  nodeColorPaletteAtom,
  hoveredNodeDataAtom,
} from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
import { continuousColor, isLightColor, withOpacity, WHITE, BLACK } from '@/services/colors';
import { useNodeColoring } from '@/hooks/useNodeColoring';
import { inferFieldFormatter } from '@/services/formatters';
import { getOperatorColor } from '@/services/query-plan/operationTypes';
import { DataText } from '@/components/ui/data-text';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  nodeId: string;
  operationType: string;
  metadata?: { rawNode?: Operator };
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}

const nodeVariants = cva(
  'px-4 py-2 rounded-md border-1 min-w-[180px] max-w-[250px] transition cursor-pointer text-foreground z-10 nodrag nopan',
  {
    variants: {
      selected: {
        true: 'shadow-glow border-2 scale-110',
        false: 'shadow-md',
      },
      dimmed: {
        true: 'opacity-30',
        false: 'opacity-100',
      },
    },
    defaultVariants: {
      selected: false,
      dimmed: false,
    },
  }
);

function nodeOpacityClass({
  hoveredStat,
  highlightedNodeIds,
  operatorId,
  isDimmed,
}: {
  hoveredStat: { values: Map<string, number> } | null | undefined;
  highlightedNodeIds: Set<string> | null;
  operatorId: string;
  isDimmed: boolean;
}): string {
  if (hoveredStat) return hoveredStat.values.has(operatorId) ? 'opacity-100' : 'opacity-20';
  // An active highlight set fully overrides the selection-based dim so that
  // hovered (highlighted) operators are always visible, even when a DAG
  // selection would otherwise dim them. The atom is fed through
  // `effectiveHighlightedNodeIdsAtom`, which clears `ids` when nothing in
  // the highlight set is actually shown — so an empty/null set here means
  // "no meaningful highlight" and we leave everything at full opacity.
  if (highlightedNodeIds !== null && highlightedNodeIds.size > 0) {
    return highlightedNodeIds.has(operatorId) ? 'opacity-100' : 'opacity-25';
  }
  if (isDimmed) return 'opacity-30';
  return 'opacity-100';
}

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  // Writes go to the source atom so the table (which reads from it directly)
  // still sees DAG hovers; reads come from the effective atom so the chart
  // doesn't dim when nothing visible would be highlighted.
  const setHighlightState = useSetAtom(highlightedNodeIdsAtom);
  const highlightState = useAtomValue(effectiveHighlightedNodeIdsAtom);
  const hoveredStat = useAtomValue(effectiveHoveredStatAtom);
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  const operatorId = data.metadata?.rawNode?.id ?? '';
  const isHighlighted = highlightState.ids !== null && highlightState.ids.has(operatorId);
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const nodeLabelField = useAtomValue(selectedNodeLabelFieldAtom);
  const { fieldColor, isDimmed, isSelected, colorField } = useNodeColoring(operatorId);
  const [isHoveredLocal, setIsHoveredLocal] = useState(false);
  const setHoveredNodeData = useSetAtom(hoveredNodeDataAtom);

  const resolvedLabel = useMemo(() => {
    if (nodeLabelField === NODE_LABEL_FIELD.ID) return data.metadata?.rawNode?.id ?? data.nodeId;
    if (nodeLabelField === NODE_LABEL_FIELD.TYPE) return data.operationType;
    return data.label;
  }, [nodeLabelField, data]);

  const colorFieldValue = colorField
    ? (statistics.find(s => s.key === colorField)?.value ?? null)
    : null;
  const formattedColorFieldValue =
    colorFieldValue === null
      ? null
      : typeof colorFieldValue === 'number'
        ? inferFieldFormatter(colorField!)(colorFieldValue)
        : String(colorFieldValue);

  const baseColor = getOperatorColor(data.operationType);
  const activeColor = fieldColor ?? baseColor;
  const bgColor =
    fieldColor ?? withOpacity(baseColor, isSelected ? 0.3 : isHoveredLocal ? 0.22 : 0.15);

  const heatmapColor = useMemo(() => {
    if (!hoveredStat) return undefined;
    const v = hoveredStat.values.get(operatorId);
    if (v === undefined) return undefined;
    const range = hoveredStat.max - hoveredStat.min;
    const t = range > 0 ? (v - hoveredStat.min) / range : 0.5;
    return continuousColor(t, nodePalette, isDarkMode);
  }, [hoveredStat, operatorId, nodePalette, isDarkMode]);

  // While a hover-driven highlight set is active, treat membership in that set
  // as the authoritative dim signal so the inner card's `dimmed` overlay does
  // not stack on top of the outer opacity for highlighted nodes.
  const hasActiveHighlight = highlightState.ids !== null;
  const effectiveDimmed = hasActiveHighlight ? !isHighlighted : isDimmed;

  const opacityClass = nodeOpacityClass({
    hoveredStat,
    highlightedNodeIds: highlightState.ids,
    operatorId,
    isDimmed: effectiveDimmed,
  });

  const isActiveHighlight = isHighlighted && !isSelected;

  const onMouseEnter = useCallback(() => {
    setIsHoveredLocal(true);
    setHoveredNodeData({
      nodeId: data.nodeId,
      label: data.label,
      operationType: data.operationType,
      statistics,
    });
    if (operatorId) {
      setHighlightState(prev => ({
        ...prev,
        ids: new Set([operatorId]),
        source: 'dag',
        primaryOperatorId: operatorId,
      }));
    }
  }, [
    data.nodeId,
    data.label,
    data.operationType,
    statistics,
    operatorId,
    setHighlightState,
    setHoveredNodeData,
  ]);
  const onMouseLeave = useCallback(() => {
    setIsHoveredLocal(false);
    setHoveredNodeData(null);
    setHighlightState(prev =>
      prev.source === 'dag' && prev.ids?.size === 1 && prev.ids.has(operatorId)
        ? { ...prev, ids: null, source: null, primaryOperatorId: null }
        : prev
    );
  }, [operatorId, setHighlightState, setHoveredNodeData]);

  const nodeContent = (
    <div
      className={nodeVariants({ selected: isSelected, dimmed: effectiveDimmed })}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={
        {
          borderColor: heatmapColor ?? activeColor,
          backgroundColor: heatmapColor ?? bgColor,
          '--glow-color': activeColor,
          ...(fieldColor && isLightColor(fieldColor) ? { color: '#111827' } : {}),
        } as React.CSSProperties
      }
    >
      {data.hasIncoming && (
        <Handle type="target" position={Position.Top} className="w-2 h-2 opacity-0" />
      )}

      <DataText
        as="div"
        className={cn('text-sm break-words text-center font-normal', {
          'font-bold': data.operationType === 'stage' || isSelected,
        })}
      >
        {resolvedLabel}
      </DataText>
      {formattedColorFieldValue !== null && (
        <div
          className="text-xs text-center mt-0.5"
          style={{
            color: fieldColor
              ? isLightColor(fieldColor)
                ? withOpacity(BLACK, 0.5)
                : withOpacity(WHITE, 0.65)
              : undefined,
          }}
        >
          {formattedColorFieldValue}
        </div>
      )}

      {data.hasOutgoing && (
        <Handle type="source" position={Position.Bottom} className="w-2 h-2 opacity-0" />
      )}
    </div>
  );

  return (
    <div
      className={cn(opacityClass, 'z-10', {
        'ring-2 ring-primary/50 rounded-md': isActiveHighlight,
      })}
    >
      {nodeContent}
    </div>
  );
});

QueryPlanNode.displayName = 'QueryPlanNode';
