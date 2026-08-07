// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Maximize2 } from 'lucide-react';
import { useSetZoomRange, useSetDebouncedZoomRange } from '@quent/hooks';
import { QueryToolbar } from './QueryToolbar';

/** Toolbar for the timeline view: shows the active operator filter and zoom reset. */
export function TimelineToolbar({ durationSeconds }: { durationSeconds: number }) {
  const setZoomRange = useSetZoomRange();
  const setDebouncedZoomRange = useSetDebouncedZoomRange();

  const resetZoom = () => {
    const full = { start: 0, end: durationSeconds };
    setZoomRange(full);
    setDebouncedZoomRange(full);
  };

  return (
    <QueryToolbar>
      <button
        onClick={resetZoom}
        className="inline-flex items-center gap-1 rounded-sm px-1.5 py-0.5 hover:bg-accent hover:text-accent-foreground transition-colors cursor-pointer"
        title="Reset zoom"
      >
        <Maximize2 className="h-3 w-3" />
        <span>Reset zoom</span>
      </button>
    </QueryToolbar>
  );
}
