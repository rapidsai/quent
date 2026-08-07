// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { DEFAULT_TIMELINE_HEIGHT, NvtxLaneChart, TimelineSkeleton } from '@quent/components';
import type { NvtxTimelineTreeItem } from './nvtxTimeline.types';

export function NvtxTreeLabel({ item }: { item: NvtxTimelineTreeItem }) {
  const metadata = item.nvtx;
  if (!metadata) return null;
  switch (metadata.kind) {
    case 'root':
      return (
        <span className="block truncate text-xs font-semibold" title={metadata.label}>
          {metadata.label}
        </span>
      );
    case 'domain':
      return (
        <span className="flex min-w-0 items-center gap-2 text-xs" title={metadata.label}>
          <span
            className="h-2.5 w-2.5 rounded-full shrink-0"
            style={{ backgroundColor: metadata.color }}
          />
          <span className="truncate">{metadata.label}</span>
        </span>
      );
    case 'lane':
      return (
        <span className="block truncate text-xs" title={metadata.label}>
          {metadata.label}
        </span>
      );
    case 'status':
      return (
        <span className="text-xs font-medium">
          {metadata.state === 'empty' ? 'No NVTX ranges in this view' : 'NVTX'}
        </span>
      );
  }
}

export function NvtxTreeUsage({
  item,
  durationSeconds,
  isDark,
}: {
  item: NvtxTimelineTreeItem;
  durationSeconds: number;
  isDark: boolean;
}) {
  const metadata = item.nvtx;
  if (!metadata) return null;
  if (metadata.kind === 'lane') {
    return (
      <NvtxLaneChart lanes={metadata.lanes} durationSeconds={durationSeconds} isDark={isDark} />
    );
  }
  if (metadata.kind !== 'status') return null;
  if (metadata.state === 'loading') {
    return (
      <div style={{ height: DEFAULT_TIMELINE_HEIGHT }} aria-label="Loading NVTX lanes…">
        <TimelineSkeleton />
      </div>
    );
  }
  return (
    <div
      className={`flex min-h-11 items-center gap-2 px-3 text-sm ${metadata.state === 'error' ? 'text-destructive' : 'text-muted-foreground'}`}
    >
      <span className="whitespace-normal">{metadata.label}</span>
      {metadata.retry && (
        <button
          type="button"
          className="rounded border border-border px-2 py-1 text-xs text-foreground hover:bg-secondary focus-visible:outline-2 focus-visible:outline-primary"
          onClick={metadata.retry}
        >
          Try again
        </button>
      )}
    </div>
  );
}
