// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Panel } from '@xyflow/react';
import { X } from 'lucide-react';
import {
  useClearInspectedPipes,
  useInspectedPipes,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
} from '@quent/hooks';

/** Floating top-right button that clears every selected operator and pipe. Hidden when nothing is selected. */
export const DAGClearSelection = () => {
  const selectedOperatorIds = useSelectedOperatorIds();
  const inspectedPipes = useInspectedPipes();
  const clearPipes = useClearInspectedPipes();
  const updateOperatorSelection = useOperatorSelectionActions();

  if (selectedOperatorIds.size === 0 && inspectedPipes.length === 0) {
    return null;
  }

  const clearSelection = () => {
    updateOperatorSelection({ type: 'clear' });
    clearPipes();
  };

  return (
    <Panel position="top-right" className="!m-2">
      <button
        type="button"
        onClick={clearSelection}
        aria-label="Clear selection"
        className="inline-flex h-6 cursor-pointer items-center gap-1 rounded-md border bg-card/90 px-2 text-xs text-muted-foreground shadow-sm backdrop-blur-sm transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <X className="size-3" />
        Clear selection
      </button>
    </Panel>
  );
};
