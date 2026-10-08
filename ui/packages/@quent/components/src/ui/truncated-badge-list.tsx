// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect, useRef, useState, type Key, type ReactNode } from 'react';
import { cn } from '@quent/utils';
import { Badge } from './badge';

export interface TruncatedBadgeListProps<T> {
  items: readonly T[];
  maxVisible: number;
  getItemKey: (item: T) => Key;
  getItemLabel: (item: T) => string;
  renderBadge: (item: T) => ReactNode;
  renderOverflowLabel?: (hiddenCount: number) => ReactNode;
  trailing?: ReactNode;
  /**
   * Keep everything on one line: show only as many badges as fit the container width
   * (up to `maxVisible`) and fold the rest into the overflow badge. The badges are
   * measured again whenever `items` changes, so pass a memoized array.
   */
  fitToWidth?: boolean;
  className?: string;
  overflowBadgeClassName?: string;
}

interface Widths {
  itemWidths: number[];
  overflowWidth: number;
  trailingWidth: number;
}

interface Measurements extends Widths {
  items: readonly unknown[];
  fitCount: number;
}

interface FitInput extends Widths {
  hasTrailing: boolean;
  available: number;
  gap: number;
  maxVisible: number;
}

function countBadgesThatFit({
  itemWidths,
  overflowWidth,
  trailingWidth,
  hasTrailing,
  available,
  gap,
  maxVisible,
}: FitInput): number {
  const total = itemWidths.length;
  const limit = Math.min(Math.max(0, maxVisible), total);
  for (let count = limit; count > 0; count -= 1) {
    const hasOverflow = count < total;
    const pieces = count + (hasOverflow ? 1 : 0) + (hasTrailing ? 1 : 0);
    const used =
      itemWidths.slice(0, count).reduce((sum, width) => sum + width, 0) +
      (hasOverflow ? overflowWidth : 0) +
      (hasTrailing ? trailingWidth : 0) +
      (pieces - 1) * gap;
    if (used <= available) {
      return count;
    }
  }
  // Always show one badge; it truncates its own label when even that is too wide.
  return Math.min(1, limit);
}

type FitKind = 'item' | 'overflow' | 'trailing';

/** In fit mode, wraps a piece in a span so its width can be measured on its own. */
function FitSlot({
  fit,
  kind,
  className,
  children,
}: {
  fit: boolean;
  kind: FitKind;
  className: string;
  children: ReactNode;
}) {
  return fit ? (
    <span data-fit={kind} className={className}>
      {children}
    </span>
  ) : (
    <>{children}</>
  );
}

function readWidth(container: HTMLElement, kind: FitKind): number {
  return container.querySelector<HTMLElement>(`[data-fit="${kind}"]`)?.offsetWidth ?? 0;
}

function readGap(container: HTMLElement): number {
  const gap = parseFloat(getComputedStyle(container).columnGap);
  return Number.isNaN(gap) ? 0 : gap;
}

export function TruncatedBadgeList<T>({
  items,
  maxVisible,
  getItemKey,
  getItemLabel,
  renderBadge,
  renderOverflowLabel = hiddenCount => `+${hiddenCount} more`,
  trailing,
  fitToWidth = false,
  className,
  overflowBadgeClassName,
}: TruncatedBadgeListProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [measurements, setMeasurements] = useState<Measurements | null>(null);
  const hasTrailing = trailing != null;

  // Show every badge once, before the browser paints, so we can read how wide each one is.
  const isMeasuring = fitToWidth && measurements?.items !== items;

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!isMeasuring || !container) {
      return;
    }
    const itemWidths = Array.from(container.querySelectorAll<HTMLElement>('[data-fit="item"]')).map(
      element => element.offsetWidth
    );
    const overflowWidth = readWidth(container, 'overflow');
    const trailingWidth = readWidth(container, 'trailing');
    setMeasurements({
      items,
      itemWidths,
      overflowWidth,
      trailingWidth,
      fitCount: countBadgesThatFit({
        itemWidths,
        overflowWidth,
        trailingWidth,
        hasTrailing,
        available: container.clientWidth,
        gap: readGap(container),
        maxVisible,
      }),
    });
  }, [isMeasuring, items, hasTrailing, maxVisible]);

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!fitToWidth || !container || typeof ResizeObserver === 'undefined') {
      return;
    }
    const observer = new ResizeObserver(() => {
      const available = container.clientWidth;
      const gap = readGap(container);
      setMeasurements(current => {
        if (!current) {
          return current;
        }
        const fitCount = countBadgesThatFit({
          ...current,
          hasTrailing,
          available,
          gap,
          maxVisible,
        });
        return fitCount === current.fitCount ? current : { ...current, fitCount };
      });
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, [fitToWidth, hasTrailing, maxVisible]);

  const visibleCount = isMeasuring
    ? items.length
    : fitToWidth && measurements
      ? Math.min(measurements.fitCount, maxVisible)
      : Math.max(0, maxVisible);
  const visibleItems = items.slice(0, visibleCount);
  const hiddenItems = items.slice(visibleItems.length);

  // While measuring, size the overflow badge for the largest count it could show.
  const overflowCount = isMeasuring ? items.length : hiddenItems.length;
  const overflowBadge =
    overflowCount > 0 ? (
      <Badge
        variant="outline"
        className={cn('shrink-0 bg-muted/40 text-muted-foreground', overflowBadgeClassName)}
        title={hiddenItems.map(getItemLabel).join(', ')}
      >
        {renderOverflowLabel(overflowCount)}
      </Badge>
    ) : null;

  return (
    <div
      ref={containerRef}
      className={cn(
        'flex items-center gap-1',
        fitToWidth ? 'flex-nowrap overflow-hidden' : 'flex-wrap',
        className
      )}
    >
      {visibleItems.map(item => (
        <FitSlot
          key={getItemKey(item)}
          fit={fitToWidth}
          kind="item"
          className={cn('flex min-w-0 max-w-full', isMeasuring ? 'shrink-0' : 'shrink')}
        >
          {renderBadge(item)}
        </FitSlot>
      ))}
      {overflowBadge && (
        <FitSlot fit={fitToWidth} kind="overflow" className="flex shrink-0">
          {overflowBadge}
        </FitSlot>
      )}
      {hasTrailing && (
        <FitSlot fit={fitToWidth} kind="trailing" className="flex shrink-0 items-center">
          {trailing}
        </FitSlot>
      )}
    </div>
  );
}
