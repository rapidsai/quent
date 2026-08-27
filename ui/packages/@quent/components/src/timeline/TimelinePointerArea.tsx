// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { forwardRef } from 'react';
import type { ComponentPropsWithoutRef } from 'react';
import { useTimelinePointerPublisher, useTimelinePointerRatio } from '@quent/hooks';
import { cn } from '@quent/utils';
import { TIMELINE_SPACING } from './types';

export type TimelinePointerRange = {
  start: number;
  end: number;
};

export type TimelinePointerAreaProps = Omit<
  ComponentPropsWithoutRef<'div'>,
  'onPointerMove' | 'onPointerLeave' | 'onPointerCancel'
> & {
  left?: number;
  right?: number;
  range?: TimelinePointerRange;
};

/** Publishes and renders the shared timeline pointer around chart content. */
export const TimelinePointerArea = forwardRef<HTMLDivElement, TimelinePointerAreaProps>(
  function TimelinePointerArea(
    {
      left = TIMELINE_SPACING.left,
      right = TIMELINE_SPACING.right,
      range,
      className,
      children,
      ...props
    },
    ref
  ) {
    const ratio = useTimelinePointerRatio();
    const { publish, clear } = useTimelinePointerPublisher();
    const rangeStart = range?.start ?? 0;
    const rangeEnd = range?.end ?? 1;
    const displayRatio = ratio == null ? null : rangeStart + ratio * (rangeEnd - rangeStart);

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
            clear();
            return;
          }
          publish((fullRatio - rangeStart) / rangeSpan);
        }}
        onPointerLeave={clear}
        onPointerCancel={clear}
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
