// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Panel } from '@xyflow/react';
import { X } from 'lucide-react';
import {
  useClearInspectedPipes,
  useDisplayedOperatorDetails,
  useDisplayedPipeDetails,
  useOperatorDetailsActions,
} from '@quent/hooks';

/** Clears every pinned DAG detail without changing operator filters. */
export const DAGClearSelection = () => {
  const operatorDetails = useDisplayedOperatorDetails();
  const pipeDetails = useDisplayedPipeDetails();
  const clearPipes = useClearInspectedPipes();
  const updateOperatorDetails = useOperatorDetailsActions();

  if (operatorDetails.length === 0 && pipeDetails.length === 0) {
    return null;
  }

  const clearSelection = () => {
    updateOperatorDetails({ type: 'clear' });
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
