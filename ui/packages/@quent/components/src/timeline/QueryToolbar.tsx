// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X, Filter } from 'lucide-react';
import {
  useSelectedOperatorLabel,
  useSetSelectedNodeIds,
  useSetSelectedOperatorLabel,
} from '@quent/hooks';

interface QueryToolbarProps {
  children?: React.ReactNode;
}

/**
 * Generic toolbar bar that shows the currently selected operator label
 * (with a clear button) on the left, and renders any provided children on
 * the right. Used by both TimelineToolbar and PivotTableToolbar.
 */
export function QueryToolbar({ children }: QueryToolbarProps) {
  const operatorLabel = useSelectedOperatorLabel();
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();

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
