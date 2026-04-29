// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtom, useSetAtom } from 'jotai';
import { Maximize2, Settings } from 'lucide-react';
import { hideTasksAtom, zoomRangeAtom, debouncedZoomRangeAtom } from '@/atoms/timeline';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { QueryToolbar } from '@/components/QueryToolbar';

export function TimelineToolbar({ durationSeconds }: { durationSeconds: number }) {
  const [hideTasks, setHideTasks] = useAtom(hideTasksAtom);
  const setZoomRange = useSetAtom(zoomRangeAtom);
  const setDebouncedZoomRange = useSetAtom(debouncedZoomRangeAtom);

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
