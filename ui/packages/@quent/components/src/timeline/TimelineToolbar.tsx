// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Maximize2, Settings } from 'lucide-react';
import {
  useHideTasks,
  useSetHideTasks,
  useSetZoomRange,
  useSetDebouncedZoomRange,
} from '@quent/hooks';
import { Popover, PopoverTrigger, PopoverContent } from '../ui/popover';
import { QueryToolbar } from './QueryToolbar';

/** Toolbar for the timeline view: shows active operator filter, zoom reset, and settings. */
export function TimelineToolbar({ durationSeconds }: { durationSeconds: number }) {
  const hideTasks = useHideTasks();
  const setHideTasks = useSetHideTasks();
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
        className="inline-flex items-center gap-1 rounded-sm px-1.5 py-0.5 hover:bg-accent hover:text-accent-foreground transition-colors"
        title="Reset zoom"
      >
        <Maximize2 className="h-3 w-3" />
        <span>Reset zoom</span>
      </button>

      <div className="h-3 w-px bg-border" />

      <Popover>
        <PopoverTrigger asChild>
          <button
            className="inline-flex items-center rounded-sm p-0.5 hover:bg-accent hover:text-accent-foreground transition-colors"
            title="Timeline settings"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>
        </PopoverTrigger>
        <PopoverContent className="text-xs">
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={hideTasks}
              onChange={e => setHideTasks(e.target.checked)}
              className="h-3 w-3 rounded-sm accent-primary cursor-pointer"
            />
            <span>Hide tasks</span>
          </label>
        </PopoverContent>
      </Popover>
    </QueryToolbar>
  );
}
