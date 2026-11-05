// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Filter, Pin } from 'lucide-react';
import { cn } from '@quent/utils';

interface DAGItemStatusProps {
  label: string;
  status: 'pinned' | 'filtered';
  className?: string;
}

export function DAGItemStatus({ label, status, className }: DAGItemStatusProps) {
  return (
    <span
      role="status"
      aria-label={`${label}: ${status}`}
      title={status}
      className={cn(
        'pointer-events-none inline-flex items-center gap-0.5 text-foreground',
        className
      )}
    >
      {status === 'filtered' ? (
        <Filter aria-hidden className="size-3.5 opacity-40" />
      ) : (
        <Pin aria-hidden className="size-3.5 opacity-40" />
      )}
    </span>
  );
}
