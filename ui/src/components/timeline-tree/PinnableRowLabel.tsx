// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { MouseEvent, ReactNode } from 'react';
import { Pin } from 'lucide-react';
import { Button } from '@quent/components';
import { cn } from '@quent/utils';
import { MAX_PINNED_ROWS } from '@/features/pinned-rows/pinnedRows';

interface PinnableRowLabelProps {
  label: ReactNode;
  // Plain-text name of the row, used to label the pin button.
  name: string;
  isPinnable: boolean;
  isPinned: boolean;
  isAtPinLimit: boolean;
  onTogglePin: () => void;
}

/** A row label with a pin button, plus where the row lives when it is shown pinned. */
export function PinnableRowLabel({
  label,
  name,
  isPinnable,
  isPinned,
  isAtPinLimit,
  onTogglePin,
}: PinnableRowLabelProps) {
  const isBlockedByLimit = isAtPinLimit && !isPinned;

  const handleClick = (event: MouseEvent<HTMLButtonElement>) => {
    // The row itself expands and selects on click; pinning should not.
    event.stopPropagation();
    onTogglePin();
  };

  return (
    <span className="flex min-w-0 items-center gap-1">
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {isPinnable && (
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          aria-label={`${isPinned ? 'Unpin' : 'Pin'} ${name}`}
          aria-pressed={isPinned}
          disabled={isBlockedByLimit}
          title={isBlockedByLimit ? `You can pin up to ${MAX_PINNED_ROWS} rows` : undefined}
          onClick={handleClick}
          // Hidden until the info area is hovered or the button is focused, unless the row is pinned.
          // Hovering the timelines does not show it.
          className={cn(
            'shrink-0 text-muted-foreground',
            !isPinned && 'opacity-0 focus-visible:opacity-100 group-hover/info:opacity-100'
          )}
        >
          <Pin className={cn(isPinned && 'fill-current')} />
        </Button>
      )}
    </span>
  );
}
