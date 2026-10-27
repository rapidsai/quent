// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Filter, Pin } from 'lucide-react';
import { cn } from '@quent/utils';

interface DAGItemStatusProps {
  label: string;
  pinned: boolean;
  filtered?: boolean;
  className?: string;
}

export function DAGItemStatus({ label, pinned, filtered = false, className }: DAGItemStatusProps) {
  if (!pinned && !filtered) {
    return null;
  }

  const state = pinned && filtered ? 'pinned and filtered' : pinned ? 'pinned' : 'filtered';

  return (
    <span
      role="status"
      aria-label={`${label}: ${state}`}
      title={state}
      className={cn(
        'pointer-events-none inline-flex h-5 items-center gap-0.5 rounded-sm border border-border/70 bg-card/90 px-1 text-primary shadow-sm backdrop-blur-sm',
        className
      )}
    >
      {pinned && <Pin aria-hidden className="size-3" />}
      {filtered && <Filter aria-hidden className="size-3" />}
    </span>
  );
}
