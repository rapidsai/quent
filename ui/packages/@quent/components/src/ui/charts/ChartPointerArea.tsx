// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { forwardRef, type ComponentPropsWithoutRef } from 'react';
import { cn } from '@quent/utils';

export interface ChartPointerRange {
  start: number;
  end: number;
}

export type ChartPointerAreaProps = Omit<
  ComponentPropsWithoutRef<'div'>,
  'onPointerMove' | 'onPointerLeave' | 'onPointerCancel'
> & {
  left: number;
  right: number;
  pointerRatio: number | null;
  onPointerRatioChange: (ratio: number | null) => void;
  range?: ChartPointerRange;
};

/** Controlled pointer overlay for aligned chart content. */
export const ChartPointerArea = forwardRef<HTMLDivElement, ChartPointerAreaProps>(
  function ChartPointerArea(
    { left, right, pointerRatio, onPointerRatioChange, range, className, children, ...props },
    ref
  ) {
    const rangeStart = range?.start ?? 0;
    const rangeEnd = range?.end ?? 1;
    const displayRatio =
      pointerRatio == null ? null : rangeStart + pointerRatio * (rangeEnd - rangeStart);

    return (
      <div
        ref={ref}
        className={cn('relative', className)}
        {...props}
        onPointerMove={event => {
          const rect = event.currentTarget.getBoundingClientRect();
          const plotWidth = rect.width - left - right;
          const plotX = event.clientX - rect.left - left;
          const fullRatio = plotX / plotWidth;
          const rangeSpan = rangeEnd - rangeStart;
          if (
            plotWidth <= 0 ||
            plotX < 0 ||
            plotX > plotWidth ||
            rangeSpan <= 0 ||
            fullRatio < rangeStart ||
            fullRatio > rangeEnd
          ) {
            onPointerRatioChange(null);
            return;
          }
          onPointerRatioChange((fullRatio - rangeStart) / rangeSpan);
        }}
        onPointerLeave={() => onPointerRatioChange(null)}
        onPointerCancel={() => onPointerRatioChange(null)}
      >
        {children}
        {displayRatio != null && (
          <div
            aria-hidden
            className="pointer-events-none absolute bottom-0 top-0 z-[10] w-0 border-l border-dashed border-muted-foreground/70"
            style={{
              left: `calc(${displayRatio * 100}% + ${left - displayRatio * (left + right)}px)`,
            }}
          />
        )}
      </div>
    );
  }
);
