// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useState, useMemo } from 'react';
import { cn } from '@/lib/utils';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import { useAtomValue, useSetAtom } from 'jotai';
import { selectedNodeLabelFieldAtom, NODE_LABEL_FIELD, hoveredNodeDataAtom } from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
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

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const operatorId = data.metadata?.rawNode?.id ?? '';
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const nodeLabelField = useAtomValue(selectedNodeLabelFieldAtom);
  const { fieldColor, isDimmed, isSelected, colorField } = useNodeColoring(operatorId);
  const [isHovered, setIsHovered] = useState(false);
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

  const baseColor = OPERATION_TYPE_COLORS[data.operationType] ?? DEFAULT_OPERATION_COLOR;
  const activeColor = fieldColor ?? baseColor;
  const bgColor = fieldColor ?? withOpacity(baseColor, isSelected ? 0.3 : isHovered ? 0.22 : 0.15);

  const nodeContent = (
    <div
      className={nodeVariants({ selected: isSelected, dimmed: isDimmed })}
      onMouseEnter={() => {
        setIsHovered(true);
        setHoveredNodeData({
          nodeId: data.nodeId,
          label: data.label,
          operationType: data.operationType,
          statistics,
        });
      }}
      onMouseLeave={() => {
        setIsHovered(false);
        setHoveredNodeData(null);
      }}
      style={
        {
          borderColor: activeColor,
          backgroundColor: bgColor,
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

  return nodeContent;
});

QueryPlanNode.displayName = 'QueryPlanNode';
