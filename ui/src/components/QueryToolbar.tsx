import { useAtomValue, useSetAtom } from 'jotai';
import { X, Filter } from 'lucide-react';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '@/atoms/dag';

interface QueryToolbarProps {
  children?: React.ReactNode;
}

export function QueryToolbar({ children }: QueryToolbarProps) {
  const operatorLabel = useAtomValue(selectedOperatorLabelAtom);
  const setSelectedNodeIds = useSetAtom(selectedNodeIdsAtom);
  const setSelectedOperatorLabel = useSetAtom(selectedOperatorLabelAtom);

  const clearOperator = () => {
    setSelectedNodeIds(new Set());
    setSelectedOperatorLabel(null);
  };

  return (
    <div className="flex items-center h-6 gap-4 px-3 py-1 border-b border-border text-xs text-muted-foreground shrink-0">
      <div className="flex items-center gap-1.5">
        <Filter className="h-3 w-3" />
        {operatorLabel ? (
          <span className="inline-flex items-center gap-1 rounded-sm bg-primary/15 text-primary px-1.5 py-0.5 font-medium">
            {operatorLabel}
            <button
              onClick={clearOperator}
              className="rounded-sm hover:bg-primary/20 p-0.5 -mr-0.5 transition-colors"
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </span>
        ) : (
          <span>No filters</span>
        )}
      </div>

      <div className="flex-1" />

      {children}
    </div>
  );
}
