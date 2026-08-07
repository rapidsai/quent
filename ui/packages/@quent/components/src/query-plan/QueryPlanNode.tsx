// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useState, useMemo, useCallback } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import {
  cn,
  continuousColor,
  isLightColor,
  withOpacity,
  getOperationTypeColor,
  WHITE,
  BLACK,
  NODE_LABEL_FIELD,
  DAG_LAYOUT_DIRECTION,
  type Operator,
  type DagLayoutDirection,
} from '@quent/utils';
import {
  useSelectedNodeLabelField,
  useNodeColoring,
  useNodeColorPalette,
  useEffectiveHighlightedNodeIds,
  useEffectiveHoveredStat,
  useSetHighlightedNodeIds,
} from '@quent/hooks';
import { formatStatWithQuantity, type QuantitySpec } from '@quent/utils';
import { parseCustomStatistics } from '../lib/queryBundle.utils';
import { DataText } from '../ui/data-text';
import { NodeFlowBar } from './NodeFlowBar';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  nodeId: string;
  operationType: string;
  metadata?: { rawNode?: Operator };
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
  /** Which edge of the node incoming/outgoing handles attach to; flips with the DAG layout direction. */
  layoutDirection?: DagLayoutDirection;
  /**
   * Whether dark mode is active. Forwarded by `DAGChart` via the node's data
   * payload so the renderer can derive heatmap colors without coupling to a
   * host theme context.
   */
  isDark?: boolean;
  /** Pre-computed collision-free color for this operator type within the current DAG. */
  baseColor?: string;
  /**
   * Whether the data-flow overlay bar is rendered under the node content.
   * Injected by `DAGChart` when converting nodes so toggling the overlay
   * relayouts exactly once.
   */
  flowBarVisible?: boolean;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}

const nodeVariants = cva(
  'w-full px-4 py-2 rounded-md border-1 transition cursor-pointer text-foreground z-10 nodrag nopan',
  {
    variants: {
      selected: {
        true: 'border-2 scale-110',
        false: '',
      },
    },
    defaultVariants: {
      selected: false,
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
    return highlightedNodeIds.has(operatorId) ? 'opacity-100' : 'opacity-35';
  }
  if (isDimmed) return 'opacity-35';
  return 'opacity-100';
}

/** Memoized DAG node rendered inside ReactFlow. */
export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  // Writes go to the source atom so the table (which reads from it directly)
  // still sees DAG hovers; reads come from the effective atom so the chart
  // doesn't dim when nothing visible would be highlighted.
  const setHighlightState = useSetHighlightedNodeIds();
  const highlightState = useEffectiveHighlightedNodeIds();
  const hoveredStat = useEffectiveHoveredStat();
  const [nodePalette] = useNodeColorPalette();
  const isDark = data.isDark ?? false;
  const operatorId = data.metadata?.rawNode?.id ?? '';
  const isHighlighted = highlightState.ids !== null && highlightState.ids.has(operatorId);
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const { quantitySpecs } = data;
  const [nodeLabelField] = useSelectedNodeLabelField();
  const { fieldColor, isDimmed, isSelected, colorField } = useNodeColoring(operatorId, isDark);
  const [isHoveredLocal, setIsHoveredLocal] = useState(false);

  const resolvedLabel = useMemo(() => {
    if (nodeLabelField === NODE_LABEL_FIELD.ID) return data.metadata?.rawNode?.id ?? data.nodeId;
    if (nodeLabelField === NODE_LABEL_FIELD.TYPE) return data.operationType;
    return data.label;
  }, [nodeLabelField, data]);

  const colorFieldStat = colorField ? statistics.find(s => s.key === colorField) : null;
  const colorFieldValue = colorFieldStat?.value ?? null;
  const formattedColorFieldValue =
    colorFieldValue === null
      ? null
      : typeof colorFieldValue === 'number'
        ? formatStatWithQuantity(
            colorFieldValue,
            colorField!,
            colorFieldStat?.quantity && quantitySpecs
              ? quantitySpecs[colorFieldStat.quantity]
              : undefined
          )
        : String(colorFieldValue);

  const baseColor = data.baseColor ?? getOperationTypeColor(data.operationType);
  const activeColor = fieldColor ?? baseColor;
  const bgColor =
    fieldColor ?? withOpacity(baseColor, isSelected ? 0.3 : isHoveredLocal ? 0.22 : 0.15);

  const heatmapColor = useMemo(() => {
    if (!hoveredStat) return undefined;
    const v = hoveredStat.values.get(operatorId);
    if (v === undefined) return undefined;
    const range = hoveredStat.max - hoveredStat.min;
    const t = range > 0 ? (v - hoveredStat.min) / range : 0.5;
    return continuousColor(t, nodePalette, isDark);
  }, [hoveredStat, operatorId, nodePalette, isDark]);

  const opacityClass = nodeOpacityClass({
    hoveredStat,
    highlightedNodeIds: highlightState.ids,
    operatorId,
    isDimmed,
  });

  const isActiveHighlight = isHighlighted && !isSelected;

  const isBottomToTop =
    (data.layoutDirection ?? DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP) ===
    DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP;
  const incomingHandlePosition = isBottomToTop ? Position.Bottom : Position.Top;
  const outgoingHandlePosition = isBottomToTop ? Position.Top : Position.Bottom;

  const onMouseEnter = useCallback(() => {
    setIsHoveredLocal(true);
    if (operatorId) {
      setHighlightState(prev => ({
        ...prev,
        ids: new Set([operatorId]),
        source: 'dag',
        primaryOperatorId: operatorId,
      }));
    }
  }, [operatorId, setHighlightState]);
  const onMouseLeave = useCallback(() => {
    setIsHoveredLocal(false);
    setHighlightState(prev =>
      prev.source === 'dag' && prev.ids?.size === 1 && prev.ids.has(operatorId)
        ? { ...prev, ids: null, source: null, primaryOperatorId: null }
        : prev
    );
  }, [operatorId, setHighlightState]);

  const nodeContent = (
    <div
      className={cn(nodeVariants({ selected: isSelected }), {
        'shadow-glow': isSelected || isActiveHighlight,
        'shadow-node': !isSelected && !isActiveHighlight,
      })}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={
        {
          borderColor: heatmapColor ?? activeColor,
          backgroundColor: heatmapColor ?? bgColor,
          '--glow-color': 'hsl(var(--primary))',
          ...(fieldColor && isLightColor(fieldColor) ? { color: '#111827' } : {}),
        } as React.CSSProperties
      }
    >
      {data.hasIncoming && (
        <Handle type="target" position={incomingHandlePosition} className="w-2 h-2 opacity-0" />
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

      {data.flowBarVisible && operatorId && <NodeFlowBar operatorId={operatorId} isDark={isDark} />}

      {data.hasOutgoing && (
        <Handle type="source" position={outgoingHandlePosition} className="w-2 h-2 opacity-0" />
      )}
    </div>
  );

  return <div className={cn(opacityClass, 'z-10')}>{nodeContent}</div>;
});

QueryPlanNode.displayName = 'QueryPlanNode';
