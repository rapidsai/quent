// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn, isLightColor } from '@quent/utils';
import { DataText } from '../ui/data-text';

/** Width-gated value label centered inside an overflow-hidden segment. */
export const SegmentValueLabel = ({
  label,
  segmentColor,
  testId,
  className,
  autoContrast = true,
}: {
  label: string;
  segmentColor: string;
  testId?: string;
  className?: string;
  autoContrast?: boolean;
}) => (
  <DataText
    data-testid={testId}
    className={cn(
      'absolute inset-0 flex items-center justify-center text-[8px] leading-none font-medium tabular-nums whitespace-nowrap',
      className
    )}
    style={
      autoContrast
        ? isLightColor(segmentColor)
          ? { color: 'rgba(0, 0, 0, 0.78)' }
          : { color: '#ffffff', textShadow: '0 0 2px rgba(0, 0, 0, 0.45)' }
        : undefined
    }
  >
    {label}
  </DataText>
);
