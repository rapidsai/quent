import { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cn } from '@/lib/utils';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  operationType: string;
  metadata?: Record<string, unknown>;
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}

const operationStyles: Record<string, string> = {
  source: 'bg-blue-100 border-blue-500 text-blue-900',
  scan: 'bg-blue-100 border-blue-500 text-blue-900',
  filesystemscan: 'bg-blue-100 border-blue-500 text-blue-900',
  join: 'bg-purple-100 border-purple-500 text-purple-900',
  joinlocal: 'bg-purple-100 border-purple-500 text-purple-900',
  joinpartition: 'bg-purple-100 border-purple-500 text-purple-900',
  aggregate: 'bg-green-100 border-green-500 text-green-900',
  exchange: 'bg-orange-100 border-orange-500 text-orange-900',
  output: 'bg-red-100 border-red-500 text-red-900',
  stage: 'bg-indigo-100 border-indigo-600 text-indigo-900 font-bold',
  local: 'bg-amber-100 border-amber-500 text-amber-900',
  project: 'bg-teal-100 border-teal-500 text-teal-900',
  filter: 'bg-cyan-100 border-cyan-500 text-cyan-900',
  sort: 'bg-violet-100 border-violet-500 text-violet-900',
  limit: 'bg-pink-100 border-pink-500 text-pink-900',
  union: 'bg-emerald-100 border-emerald-500 text-emerald-900',
  other: 'bg-gray-100 border-gray-500 text-gray-900',
  default: 'bg-gray-100 border-gray-500 text-gray-900',
};

export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const styleClass = operationStyles[data.operationType] || operationStyles.other;

  return (
    <div
      className={cn(
        'px-4 py-2 rounded-lg border-2 shadow-md min-w-[180px] max-w-[250px]',
        styleClass
      )}
      style={{ zIndex: 10 }}
    >
      {data.hasIncoming && (
        <Handle type="target" position={Position.Top} className="w-2 h-2" style={{ opacity: 0 }} />
      )}

      <div className="text-sm font-semibold break-words text-center">{data.label}</div>

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
});

QueryPlanNode.displayName = 'QueryPlanNode';
