// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { forwardRef, useCallback, type ComponentPropsWithoutRef } from 'react';
import { useTimelinePointerPublisher, useTimelinePointerRatio } from '@quent/hooks';
import { ChartPointerArea } from '../ui/charts/ChartPointerArea';
import type { ChartPointerRange } from '../ui/charts/ChartPointerArea';
import { TIMELINE_SPACING } from './types';

export type TimelinePointerRange = ChartPointerRange;

export type TimelinePointerAreaProps = Omit<
  ComponentPropsWithoutRef<'div'>,
  'onPointerMove' | 'onPointerLeave' | 'onPointerCancel'
> & {
  left?: number;
  right?: number;
  range?: TimelinePointerRange;
};

/** Hook-backed adapter for the controlled chart pointer overlay. */
export const TimelinePointerArea = forwardRef<HTMLDivElement, TimelinePointerAreaProps>(
  function TimelinePointerArea(
    { left = TIMELINE_SPACING.left, right = TIMELINE_SPACING.right, ...props },
    ref
  ) {
    const pointerRatio = useTimelinePointerRatio();
    const { publish, clear } = useTimelinePointerPublisher();
    const handlePointerRatioChange = useCallback(
      (ratio: number | null) => {
        if (ratio == null) {
          clear();
        } else {
          publish(ratio);
        }
      },
      [clear, publish]
    );

    return (
      <ChartPointerArea
        {...props}
        ref={ref}
        left={left}
        right={right}
        pointerRatio={pointerRatio}
        onPointerRatioChange={handlePointerRatioChange}
      />
    );
  }
);
