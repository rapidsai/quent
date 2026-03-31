// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import { useAtomValue } from 'jotai';
import { selectedNodeIdsAtom } from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
import { OperatorStatisticsPopup } from './OperatorStatisticsPopup';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
import { OPERATION_TYPE_COLORS, DEFAULT_OPERATION_COLOR } from '@/services/query-plan/operationTypes';

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

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const isSelected = selectedNodeIds.has(data.metadata?.rawNode?.id ?? '');
  const hasSelection = selectedNodeIds.size > 0;
  const isDimmed = hasSelection && !isSelected;
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const [isHovered, setIsHovered] = useState(false);

  const color = OPERATION_TYPE_COLORS[data.operationType] ?? DEFAULT_OPERATION_COLOR;
  const bgOpacity = isSelected ? '4d' : isHovered ? '38' : '26';

  const nodeContent = (
    <div
      className={nodeVariants({ selected: isSelected, dimmed: isDimmed })}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      style={{
        borderColor: color,
        backgroundColor: color + bgOpacity,
        '--glow-color': color,
      } as React.CSSProperties}
    >
      {data.hasIncoming && (
        <Handle type="target" position={Position.Top} className="w-2 h-2" style={{ opacity: 0 }} />
      )}

      <div
        className={`text-sm break-words text-center ${data.operationType === 'stage' ? 'font-bold' : isSelected ? 'font-bold' : 'font-normal'}`}
      >
        {data.label}
      </div>

      {data.hasOutgoing && (
        <Handle
          type="source"
          position={Position.Bottom}
          className="w-2 h-2"
          style={{ opacity: 0 }}
        />
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
      {nodeContent}
    </OperatorStatisticsPopup>
  );
});

QueryPlanNode.displayName = 'QueryPlanNode';
