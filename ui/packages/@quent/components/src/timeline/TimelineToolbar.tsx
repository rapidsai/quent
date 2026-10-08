// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Maximize2 } from 'lucide-react';
import { useSetZoomRange, useSetDebouncedZoomRange } from '@quent/hooks';
import { Button } from '../ui/button';
import { TimelineSettingsPopover } from './TimelineSettingsPopover';

interface TimelineActionsProps {
  durationSeconds: number;
}

export function TimelineActions({ durationSeconds }: TimelineActionsProps) {
  const setZoomRange = useSetZoomRange();
  const setDebouncedZoomRange = useSetDebouncedZoomRange();

  const resetZoom = () => {
    const full = { start: 0, end: durationSeconds };
    setZoomRange(full);
    setDebouncedZoomRange(full);
  };

  return (
    <>
      <Button type="button" variant="ghost" size="xs" onClick={resetZoom} title="Reset zoom">
        <Maximize2 />
        Reset zoom
      </Button>
      <div className="h-3 w-px bg-border" />
      <TimelineSettingsPopover />
    </>
  );
}

interface TimelineToolbarProps extends TimelineActionsProps {
  filters?: React.ReactNode;
}

export function TimelineToolbar({ durationSeconds, filters }: TimelineToolbarProps) {
  return (
    <div className="flex min-h-8 shrink-0 items-center gap-4 border-b border-border px-3 py-1 text-xs text-muted-foreground">
      <div className="flex min-w-0 flex-1 items-center gap-1.5">{filters}</div>
      <div className="flex shrink-0 items-center gap-2">
        <TimelineActions durationSeconds={durationSeconds} />
      </div>
    </div>
  );
}
