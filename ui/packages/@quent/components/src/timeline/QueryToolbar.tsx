// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, type ReactNode } from 'react';
import { X, Filter } from 'lucide-react';
import { useOperatorSelection, useOperatorSelectionActions } from '@quent/hooks';
import { Badge } from '../ui/badge';
import { TruncatedBadgeList } from '../ui/truncated-badge-list';

const MAX_VISIBLE_OPERATOR_BADGES = 3;

interface QueryToolbarProps {
  children?: ReactNode;
  filters?: ReactNode;
}

/**
 * Generic query toolbar with filter controls on the left and actions on the right.
 */
export function QueryToolbar({ children, filters }: QueryToolbarProps) {
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();

  const selectedOperators = useMemo(
    () =>
      Array.from(operatorSelection.selections, ([id, selection]) => ({
        id,
        label: selection.label,
      })),
    [operatorSelection.selections]
  );

  const clearOperators = () => {
    updateOperatorSelection({ type: 'clear' });
  };

  const removeOperator = (operatorId: string) => {
    updateOperatorSelection({ type: 'remove', selectionId: operatorId });
  };

  return (
    <div className="flex min-h-8 items-center gap-4 border-b border-border px-3 py-1 text-xs text-muted-foreground shrink-0">
      <div className="flex min-w-0 flex-1 items-center gap-1.5">
        {filters}
        <div
          className="flex min-w-0 max-w-[40%] items-center gap-1.5 overflow-hidden"
          data-testid="operator-filter-badges"
        >
          <Filter className="h-3 w-3 shrink-0" />
          {selectedOperators.length > 0 ? (
            <TruncatedBadgeList
              items={selectedOperators}
              maxVisible={MAX_VISIBLE_OPERATOR_BADGES}
              getItemKey={operator => operator.id}
              getItemLabel={operator => operator.label}
              className="min-w-0 flex-nowrap overflow-hidden"
              overflowBadgeClassName="px-1.5 py-0"
              renderOverflowLabel={hiddenCount => `and ${hiddenCount} more`}
              renderBadge={operator => (
                <Badge
                  variant="outline"
                  className="min-w-0 max-w-36 shrink bg-primary/10 px-1.5 py-0 font-medium text-primary"
                  title={operator.label}
                >
                  <span className="truncate">{operator.label}</span>
                  <button
                    type="button"
                    onClick={() => removeOperator(operator.id)}
                    aria-label={`Remove ${operator.label}`}
                    className="ml-0.5 shrink-0 cursor-pointer rounded-sm opacity-70 hover:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  >
                    <X className="size-2.5" />
                  </button>
                </Badge>
              )}
            />
          ) : !filters ? (
            <span className="shrink-0">No filters</span>
          ) : null}
          {selectedOperators.length > 0 && (
            <button
              type="button"
              onClick={clearOperators}
              aria-label="Clear all operator filters"
              className="shrink-0 cursor-pointer rounded-sm px-1 py-0.5 font-medium text-primary hover:bg-primary/10 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              Clear
            </button>
          )}
        </div>
      </div>

      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}
