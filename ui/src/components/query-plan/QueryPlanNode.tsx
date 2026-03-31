import { memo, useMemo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import { useAtomValue } from 'jotai';
import { selectedNodeIdsAtom, nodeColoringAtom, selectedNodeDisplayFieldAtom, nodeColorPaletteAtom, selectedNodeLabelFieldAtom, NODE_LABEL_FIELD } from '@/atoms/dag';
import { Operator } from '~quent/types/Operator';
import { OperatorStatisticsPopup } from './OperatorStatisticsPopup';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
import { continuousColor } from '@/services/colors';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  nodeId: string;
  operationType: string;
  metadata?: { rawNode?: Operator };
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}

const nodeVariants = cva(
  'px-4 py-2 rounded-md border-1 min-w-[180px] max-w-[250px] transition-colors cursor-pointer text-foreground',
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
        true: 'shadow-glow',
        false: 'shadow-md',
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
  const nodeColoring = useAtomValue(nodeColoringAtom);
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const operatorId = data.metadata?.rawNode?.id ?? '';
  const isSelected = selectedNodeIds.has(operatorId);
  const statistics = parseCustomStatistics(data.metadata?.rawNode);
  const nodeDisplayField = useAtomValue(selectedNodeDisplayFieldAtom);
  const nodeLabelField = useAtomValue(selectedNodeLabelFieldAtom);

  const resolvedLabel = useMemo(() => {
    if (nodeLabelField === NODE_LABEL_FIELD.ID) return data.metadata?.rawNode?.id ?? data.nodeId;
    if (nodeLabelField === NODE_LABEL_FIELD.TYPE) return data.operationType;
    return data.label;
  }, [nodeLabelField, data]);

  const { fieldColor, fieldDimmed } = useMemo(() => {
    if (!nodeColoring) return { fieldColor: undefined, fieldDimmed: false };
    if (nodeColoring.type === 'continuous') {
      const v = nodeColoring.values.get(operatorId);
      if (v === undefined) return { fieldColor: undefined, fieldDimmed: true };
      const t =
        nodeColoring.max > nodeColoring.min
          ? (v - nodeColoring.min) / (nodeColoring.max - nodeColoring.min)
          : 0.5;
      return { fieldColor: continuousColor(t, nodePalette), fieldDimmed: false };
    }
    const color = nodeColoring.colorMap.get(operatorId);
    return { fieldColor: color, fieldDimmed: !color };
  }, [nodeColoring, operatorId, nodePalette]);

  const nodeContent = (
    <div
      className={nodeVariants({
        operationType: resolveOperationType(data.operationType),
        selected: isSelected,
      })}
      style={{
        zIndex: 10,
        opacity: fieldDimmed ? 0.25 : 1,
        transition: 'opacity 150ms, background-color 150ms, border-color 150ms',
        ...(fieldColor && { backgroundColor: fieldColor, borderColor: fieldColor }),
      }}
    >
      {data.hasIncoming && (
        <Handle type="target" position={Position.Top} className="w-2 h-2" style={{ opacity: 0 }} />
      )}

      <div className="text-sm font-normal break-words text-center">{resolvedLabel}</div>
      {nodeDisplayField && (() => {
        const displayValue = statistics.find(s => s.key === nodeDisplayField)?.value ?? null;
        return displayValue !== null ? (
          <div className="text-xs text-muted-foreground text-center mt-0.5">
            {String(displayValue)}
          </div>
        ) : null;
      })()}

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
