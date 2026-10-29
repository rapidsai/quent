// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Filter, Pin, PinOff } from 'lucide-react';
import { cn } from '@quent/utils';

interface DetailItemActionsProps {
  label: string;
  filtered?: boolean;
  pinned: boolean;
  onToggleFilter?: () => void;
  onTogglePin?: () => void;
  onUnpin?: () => void;
}

const actionClass =
  'inline-flex size-6 cursor-pointer items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-muted-foreground';

export function DetailItemActions({
  label,
  filtered = false,
  pinned,
  onToggleFilter,
  onTogglePin,
  onUnpin,
}: DetailItemActionsProps) {
  return (
    <div className="flex shrink-0 items-center gap-0.5">
      {onToggleFilter && (
        <button
          type="button"
          aria-label={`Filter ${label}`}
          aria-pressed={filtered}
          title={filtered ? 'Remove filter' : 'Filter'}
          className={cn(actionClass, filtered && 'bg-primary/10 text-primary')}
          onClick={onToggleFilter}
        >
          <Filter aria-hidden className="size-3.5" />
        </button>
      )}
      {onTogglePin && (
        <button
          type="button"
          aria-label={`Pin ${label}`}
          aria-pressed={pinned}
          title={pinned ? 'Unpin' : 'Pin'}
          className={cn(actionClass, pinned && 'bg-primary/10 text-primary')}
          onClick={onTogglePin}
        >
          <Pin aria-hidden className="size-3.5" />
        </button>
      )}
      {onUnpin && (
        <button
          type="button"
          aria-label={`Unpin ${label}`}
          title="Unpin"
          className={actionClass}
          onClick={onUnpin}
        >
          <PinOff aria-hidden className="size-3.5" />
        </button>
      )}
    </div>
  );
}
