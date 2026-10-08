// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, type ReactNode } from 'react';
import { Filter } from 'lucide-react';
import {
  useOperatorSelection,
  useOperatorSelectionActions,
  useSelectedOperatorsData,
} from '@quent/hooks';
import { cn } from '@quent/utils';
import { Button } from '../ui/button';
import { TruncatedBadgeList } from '../ui/truncated-badge-list';
import { OperatorBadge } from './OperatorBadge';

const MAX_VISIBLE_OPERATOR_BADGES = 12;

interface QueryToolbarProps {
  children?: ReactNode;
}

/** Selected-operator filters on the left; `children` are query-wide actions on the right. */
export function QueryToolbar({ children }: QueryToolbarProps) {
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();

  const selectedOperatorsData = useSelectedOperatorsData();
  const selectedOperators = useMemo(() => {
    const operationTypeById = new Map(
      selectedOperatorsData.map(data => [data.nodeId, data.operationType])
    );
    return Array.from(operatorSelection.selections, ([id, selection]) => ({
      id,
      label: selection.label,
      operationType: operationTypeById.get(id),
    }));
  }, [operatorSelection.selections, selectedOperatorsData]);

  const clearOperators = () => {
    updateOperatorSelection({ type: 'clear' });
  };

  const removeOperator = (operatorId: string) => {
    updateOperatorSelection({ type: 'remove', selectionId: operatorId });
  };

  const hasSelection = selectedOperators.length > 0;

  return (
    <div className="flex min-h-11 shrink-0 items-start gap-4 border-b border-border px-4 py-1.5 text-sm text-muted-foreground">
      <div className="flex min-w-0 max-w-[60%] flex-1 items-start gap-2">
        <span className="flex h-8 shrink-0 items-center">
          <Filter
            className={cn(
              'size-4 transition-colors',
              hasSelection ? 'text-primary' : 'text-muted-foreground'
            )}
          />
        </span>
        {hasSelection ? (
          <TruncatedBadgeList
            items={selectedOperators}
            maxVisible={MAX_VISIBLE_OPERATOR_BADGES}
            fitToWidth
            getItemKey={operator => operator.id}
            getItemLabel={operator => operator.label}
            className="h-8 min-w-0 flex-1 gap-2"
            overflowBadgeClassName="h-6 rounded-md px-2 py-0 text-sm"
            renderOverflowLabel={hiddenCount => `and ${hiddenCount} more`}
            trailing={
              <div className="flex items-center gap-1.5">
                <div className="h-4 w-px shrink-0 bg-border" />
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  onClick={clearOperators}
                  aria-label="Clear all filters"
                >
                  Clear
                </Button>
              </div>
            }
            renderBadge={operator => (
              <OperatorBadge
                label={operator.label}
                operationType={operator.operationType}
                onRemove={() => removeOperator(operator.id)}
              />
            )}
          />
        ) : (
          <span className="flex h-8 items-center">No active filters</span>
        )}
      </div>

      {children && <div className="ml-auto flex h-8 shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}
