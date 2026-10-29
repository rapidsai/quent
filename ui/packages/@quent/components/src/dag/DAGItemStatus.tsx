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
      className={cn('pointer-events-none inline-flex items-center gap-0.5 text-primary', className)}
    >
      {filtered && <Filter aria-hidden className="size-3.5" />}
      {pinned && !filtered && <Pin aria-hidden className="size-3.5" />}
    </span>
  );
}
