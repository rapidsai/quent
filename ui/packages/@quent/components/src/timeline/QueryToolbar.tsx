// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X, Filter } from 'lucide-react';
import {
  useSelectedOperatorLabel,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
  useSetSelectedNodeData,
} from '@quent/hooks';

interface QueryToolbarProps {
  children?: React.ReactNode;
  filters?: React.ReactNode;
}

/**
 * Generic query toolbar with filter controls on the left and actions on the right.
 */
export function QueryToolbar({ children, filters }: QueryToolbarProps) {
  const operatorLabel = useSelectedOperatorLabel();
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();
  const setSelectedNodeData = useSetSelectedNodeData();

  const clearOperator = () => {
    setSelectedNodeIds(new Set());
    setSelectedOperatorLabel(null);
    setSelectedNodeData(null);
  };

  return (
    <div className="flex min-h-8 items-center gap-4 border-b border-border px-3 py-1 text-xs text-muted-foreground shrink-0">
      <div className="flex min-w-0 flex-1 items-center gap-1.5">
        {filters}
        <Filter className="h-3 w-3 shrink-0" />
        {operatorLabel ? (
          <span className="inline-flex shrink-0 items-center gap-1 rounded-sm bg-primary/15 text-primary px-1.5 py-0.5 font-medium">
            {operatorLabel}
            <button
              aria-label={`Clear operator filter ${operatorLabel}`}
              onClick={clearOperator}
              className="rounded-sm hover:bg-primary/20 p-0.5 -mr-0.5 transition-colors cursor-pointer"
              type="button"
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </span>
        ) : !filters ? (
          <span>No filters</span>
        ) : null}
      </div>

      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}
