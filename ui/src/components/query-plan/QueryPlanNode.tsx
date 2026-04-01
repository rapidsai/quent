// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import { useAtomValue } from 'jotai';
import { selectedNodeIdsAtom } from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
import { OperatorStatisticsPopup } from './OperatorStatisticsPopup';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
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
      operationType: {
        source:
          'bg-blue-100/15 border-blue-500 hover:bg-blue-100/30 [--glow-color:var(--color-blue-500)]',
        scan: 'bg-blue-100/15 border-blue-500 hover:bg-blue-100/30 [--glow-color:var(--color-blue-500)]',
        filesystemscan:
          'bg-blue-100/15 border-blue-500 hover:bg-blue-100/30 [--glow-color:var(--color-blue-500)]',
        join: 'bg-purple-100/15 border-purple-500 hover:bg-purple-100/30 [--glow-color:var(--color-purple-500)]',
        joinlocal:
          'bg-purple-100/15 border-purple-500 hover:bg-purple-100/30 [--glow-color:var(--color-purple-500)]',
        joinpartition:
          'bg-purple-100/15 border-purple-500 hover:bg-purple-100/30 [--glow-color:var(--color-purple-500)]',
        aggregate:
          'bg-green-100/15 border-green-500 hover:bg-green-100/30 [--glow-color:var(--color-green-500)]',
        exchange:
          'bg-orange-100/15 border-orange-500 hover:bg-orange-100/30 [--glow-color:var(--color-orange-500)]',
        output:
          'bg-red-100/15 border-red-500 hover:bg-red-100/30 [--glow-color:var(--color-red-500)]',
        stage:
          'bg-indigo-100/15 border-indigo-600 hover:bg-indigo-100/30 [--glow-color:var(--color-indigo-600)] font-bold',
        local:
          'bg-amber-100/15 border-amber-500 hover:bg-amber-100/30 [--glow-color:var(--color-amber-500)]',
        project:
          'bg-teal-100/15 border-teal-500 hover:bg-teal-100/30 [--glow-color:var(--color-teal-500)]',
        filter:
          'bg-cyan-100/15 border-cyan-500 hover:bg-cyan-100/30 [--glow-color:var(--color-cyan-500)]',
        sort: 'bg-violet-100/15 border-violet-500 hover:bg-violet-100/30 [--glow-color:var(--color-violet-500)]',
        limit:
          'bg-pink-100/15 border-pink-500 hover:bg-pink-100/30 [--glow-color:var(--color-pink-500)]',
        union:
          'bg-emerald-100/15 border-emerald-500 hover:bg-emerald-100/30 [--glow-color:var(--color-emerald-500)]',
        other:
          'bg-gray-100/15 border-gray-500 hover:bg-gray-100/30 [--glow-color:var(--color-gray-500)]',
      },
      selected: {
        true: 'shadow-glow border-2 scale-110',
        false: 'shadow-md',
      },
      dimmed: {
        true: 'opacity-30',
        false: 'opacity-100',
      },
    },
    compoundVariants: [
      {
        operationType: ['source', 'scan', 'filesystemscan'],
        selected: true,
        class: 'bg-blue-100/30',
      },
      {
        operationType: ['join', 'joinlocal', 'joinpartition'],
        selected: true,
        class: 'bg-purple-100/30',
      },
      { operationType: 'aggregate', selected: true, class: 'bg-green-100/30' },
      { operationType: 'exchange', selected: true, class: 'bg-orange-100/30' },
      { operationType: 'output', selected: true, class: 'bg-red-100/30' },
      { operationType: 'stage', selected: true, class: 'bg-indigo-100/30' },
      { operationType: 'local', selected: true, class: 'bg-amber-100/30' },
      { operationType: 'project', selected: true, class: 'bg-teal-100/30' },
      { operationType: 'filter', selected: true, class: 'bg-cyan-100/30' },
      { operationType: 'sort', selected: true, class: 'bg-violet-100/30' },
      { operationType: 'limit', selected: true, class: 'bg-pink-100/30' },
      { operationType: 'union', selected: true, class: 'bg-emerald-100/30' },
      { operationType: 'other', selected: true, class: 'bg-gray-100/30' },
    ],
    defaultVariants: {
      operationType: 'other',
      selected: false,
      dimmed: false,
    },
  }
);

type OperationType = NonNullable<Parameters<typeof nodeVariants>[0]>['operationType'];

const validOperationTypes: Set<string> = new Set([
  'source',
  'scan',
  'filesystemscan',
  'join',
  'joinlocal',
  'joinpartition',
  'aggregate',
  'exchange',
  'output',
  'stage',
  'local',
  'project',
  'filter',
  'sort',
  'limit',
  'union',
  'other',
]);

function resolveOperationType(type: string): OperationType {
  return (validOperationTypes.has(type) ? type : 'other') as OperationType;
}

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const isSelected = selectedNodeIds.has(data.metadata?.rawNode?.id ?? '');
  const hasSelection = selectedNodeIds.size > 0;
  const isDimmed = hasSelection && !isSelected;
  const statistics = parseCustomStatistics(data.metadata?.rawNode);

  const nodeContent = (
    <div
      className={nodeVariants({
        operationType: resolveOperationType(data.operationType),
        selected: isSelected,
        dimmed: isDimmed,
      })}
    >
      {data.hasIncoming && (
        <Handle type="target" position={Position.Top} className="w-2 h-2" style={{ opacity: 0 }} />
      )}

      <DataText
        as="div"
        className={`text-sm break-words text-center ${isSelected ? 'font-bold' : 'font-normal'}`}
      >
        {data.label}
      </DataText>

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
