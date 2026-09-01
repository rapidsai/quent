// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Fragment, type Key, type ReactNode } from 'react';
import { cn } from '@quent/utils';
import { Badge } from './badge';

export interface TruncatedBadgeListProps<T> {
  items: readonly T[];
  maxVisible: number;
  getItemKey: (item: T) => Key;
  getItemLabel: (item: T) => string;
  renderBadge: (item: T) => ReactNode;
  renderOverflowLabel?: (hiddenCount: number) => ReactNode;
  className?: string;
  overflowBadgeClassName?: string;
}

export function TruncatedBadgeList<T>({
  items,
  maxVisible,
  getItemKey,
  getItemLabel,
  renderBadge,
  renderOverflowLabel = hiddenCount => `+${hiddenCount} more`,
  className,
  overflowBadgeClassName,
}: TruncatedBadgeListProps<T>) {
  const visibleItems = items.slice(0, Math.max(0, maxVisible));
  const hiddenItems = items.slice(visibleItems.length);

  return (
    <div className={cn('flex flex-wrap items-center gap-1', className)}>
      {visibleItems.map(item => (
        <Fragment key={getItemKey(item)}>{renderBadge(item)}</Fragment>
      ))}
      {hiddenItems.length > 0 && (
        <Badge
          variant="outline"
          className={cn('shrink-0 bg-muted/40 text-muted-foreground', overflowBadgeClassName)}
          title={hiddenItems.map(getItemLabel).join(', ')}
        >
          {renderOverflowLabel(hiddenItems.length)}
        </Badge>
      )}
    </div>
  );
}
