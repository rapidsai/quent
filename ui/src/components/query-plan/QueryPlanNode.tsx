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
  hoveredOperatorIdAtom,
  hoveredOperatorInfoAtom,
  hoveredStatAtom,
  hoveredOperatorTypeAtom,
  highlightedNodeIdsAtom,
} from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
import { OperatorStatisticsPopup } from './OperatorStatisticsPopup';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
import { isLightColor, withOpacity, WHITE, BLACK } from '@/services/colors';
import { useNodeColoring } from '@/hooks/useNodeColoring';
import { inferFieldFormatter } from '@/services/query-plan/dagFieldProcessing';
import {
  OPERATION_TYPE_COLORS,
  DEFAULT_OPERATION_COLOR,
} from '@/services/query-plan/operationTypes';
import { DataText } from '@/components/ui/data-text';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  nodeId: string;
  operationType: string;
  metadata?: { rawNode?: Operator };
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}

const nodeVariants = cva(
  'px-4 py-2 rounded-md border-1 min-w-[180px] max-w-[250px] transition cursor-pointer text-foreground z-10',
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

/** Same red gradient as the table cells, but with higher alpha for node backgrounds. */
const GRADIENT_COLOR: [number, number, number] = [239, 68, 68]; // red-500
function heatmapBg(t: number): string {
  const alpha = 0.1 + t * 0.55; // 0.1 at low → 0.65 at high
  return `rgba(${GRADIENT_COLOR[0]}, ${GRADIENT_COLOR[1]}, ${GRADIENT_COLOR[2]}, ${alpha.toFixed(3)})`;
}

function nodeOpacityClass({
  hoveredStat,
  hoveredOperatorId,
  hoveredOpType,
  highlightedNodeIds,
  operatorId,
  isHovered,
  isTypeHovered,
  isHighlighted,
  isDimmed,
}: {
  hoveredStat: { values: Map<string, number> } | null | undefined;
  hoveredOperatorId: string | null;
  hoveredOpType: string | null;
  highlightedNodeIds: Set<string> | null;
  operatorId: string;
  isHovered: boolean;
  isTypeHovered: boolean;
  isHighlighted: boolean;
  isDimmed: boolean;
}): string {
  if (hoveredStat) return hoveredStat.values.has(operatorId) ? 'opacity-100' : 'opacity-20';
  if (highlightedNodeIds !== null && !isHighlighted) return 'opacity-25';
  if (hoveredOpType !== null && !isTypeHovered) return 'opacity-25';
  if (hoveredOperatorId !== null && !isHovered) return 'opacity-25';
  if (isDimmed) return 'opacity-30';
  return 'opacity-100';
}

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const hoveredOperatorId = useAtomValue(hoveredOperatorIdAtom);
  const setHoveredOperatorId = useSetAtom(hoveredOperatorIdAtom);
  const setHoveredOperatorInfo = useSetAtom(hoveredOperatorInfoAtom);
  const hoveredStat = useAtomValue(hoveredStatAtom);
  const hoveredOpType = useAtomValue(hoveredOperatorTypeAtom);
  const highlightedNodeIds = useAtomValue(highlightedNodeIdsAtom);
  const operatorId = data.metadata?.rawNode?.id ?? '';
  const operatorTypeName = data.metadata?.rawNode?.operator_type_name ?? data.operationType;
  const isHoveredFromTable = hoveredOperatorId === operatorId && operatorId !== '';
  const isTypeHovered =
    hoveredOpType !== null &&
    hoveredOpType.toLowerCase().split(', ').includes(operatorTypeName.toLowerCase());
  const isHighlighted = highlightedNodeIds !== null && highlightedNodeIds.has(operatorId);
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const nodeLabelField = useAtomValue(selectedNodeLabelFieldAtom);
  const { fieldColor, isDimmed, isSelected, colorField } = useNodeColoring(operatorId);
  const [isHoveredLocal, setIsHoveredLocal] = useState(false);

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

  const baseColor = OPERATION_TYPE_COLORS[data.operationType] ?? DEFAULT_OPERATION_COLOR;
  const activeColor = fieldColor ?? baseColor;
  const bgColor =
    fieldColor ?? withOpacity(baseColor, isSelected ? 0.3 : isHoveredLocal ? 0.22 : 0.15);

  const heatmapColor = useMemo(() => {
    if (!hoveredStat) return undefined;
    const v = hoveredStat.values.get(operatorId);
    if (v === undefined) return undefined;
    const range = hoveredStat.max - hoveredStat.min;
    const t = range > 0 ? (v - hoveredStat.min) / range : 0.5;
    return heatmapBg(t);
  }, [hoveredStat, operatorId]);

  const opacityClass = nodeOpacityClass({
    hoveredStat,
    hoveredOperatorId,
    hoveredOpType,
    highlightedNodeIds,
    operatorId,
    isHovered: isHoveredFromTable,
    isTypeHovered,
    isHighlighted,
    isDimmed,
  });

  const isActiveHighlight = (isHoveredFromTable || isTypeHovered || isHighlighted) && !isSelected;

  const onMouseEnter = useCallback(() => {
    setIsHoveredLocal(true);
    if (operatorId) {
      setHoveredOperatorId(operatorId);
      setHoveredOperatorInfo({
        nodeId: data.nodeId,
        label: data.label,
        operationType: data.metadata?.rawNode?.operator_type_name ?? data.operationType,
        stats: statistics,
      });
    }
  }, [
    operatorId,
    setHoveredOperatorId,
    setHoveredOperatorInfo,
    data.nodeId,
    data.label,
    data.metadata?.rawNode?.operator_type_name,
    data.operationType,
    statistics,
  ]);
  const onMouseLeave = useCallback(() => {
    setIsHoveredLocal(false);
    setHoveredOperatorId(prev => (prev === operatorId ? null : prev));
    setHoveredOperatorInfo(prev => (prev?.nodeId === data.nodeId ? null : prev));
  }, [operatorId, setHoveredOperatorId, setHoveredOperatorInfo, data.nodeId, setIsHoveredLocal]);

  const nodeContent = (
    <div
      className={nodeVariants({ selected: isSelected, dimmed: isDimmed })}
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
    <OperatorStatisticsPopup
      data={statistics}
      nodeId={data.nodeId}
      operatorLabel={data.label}
      operationType={data.operationType}
    >
      <div className={cn(opacityClass, 'z-10', { 'ring-2 ring-primary/50 rounded-md': isActiveHighlight })}>
        {nodeContent}
      </div>
    </OperatorStatisticsPopup>
  );
});

QueryPlanNode.displayName = 'QueryPlanNode';
