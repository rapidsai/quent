// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState, type CSSProperties, type ReactNode } from 'react';
import { cn } from '@quent/utils';
import { PointerTooltipPortal, type PointerPosition } from '../ui/pointer-tooltip-portal';
import { SegmentValueLabel } from './SegmentValueLabel';

export interface SegmentedBarSegment {
  id: string;
  value: number;
  color: string;
  label?: string;
  labelClassName?: string;
  autoLabelContrast?: boolean;
  tooltip?: ReactNode;
  ariaLabel?: string;
  title?: string;
}

export interface SegmentedBarProps {
  segments: SegmentedBarSegment[];
  fillValue?: number;
  maxValue?: number;
  height?: number | string;
  minimumFillPx?: number;
  showLabels?: boolean;
  showTooltips?: boolean;
  transition?: string;
  className?: string;
  trackClassName?: string;
  labelTestId?: string;
  style?: CSSProperties;
}

export function SegmentedBar({
  segments,
  fillValue,
  maxValue,
  height = 12,
  minimumFillPx = 0,
  showLabels = true,
  showTooltips = true,
  transition,
  className,
  trackClassName,
  labelTestId,
  style,
}: SegmentedBarProps) {
  const [tooltip, setTooltip] = useState<{
    content: ReactNode;
    pointer: PointerPosition;
  } | null>(null);
  const total = segments.reduce((sum, segment) => sum + Math.max(0, segment.value), 0);
  const filledValue = fillValue ?? total;
  const scaleMax = maxValue ?? filledValue;
  const fillPercent = scaleMax > 0 ? Math.min(100, (filledValue / scaleMax) * 100) : 0;
  const fillWidth =
    filledValue > 0 && minimumFillPx > 0
      ? `max(${minimumFillPx}px, ${fillPercent}%)`
      : `${fillPercent}%`;

  const showSegmentTooltip = (content: ReactNode, pointer: PointerPosition) => {
    if (showTooltips) setTooltip({ content, pointer });
  };

  return (
    <>
      <div
        className={cn('w-full overflow-hidden rounded-sm bg-muted/40', trackClassName, className)}
        style={{ ...style, height }}
      >
        <div className="flex h-full" style={{ width: fillWidth, transition }}>
          {segments.map(segment => (
            <div
              key={segment.id}
              role={segment.ariaLabel ? 'img' : undefined}
              aria-label={segment.ariaLabel}
              title={segment.title}
              tabIndex={showTooltips && segment.tooltip ? 0 : undefined}
              className="relative min-w-0 basis-0 overflow-hidden focus-visible:brightness-90"
              style={{ flexGrow: Math.max(0, segment.value), backgroundColor: segment.color }}
              onMouseEnter={event => {
                if (segment.tooltip) {
                  showSegmentTooltip(segment.tooltip, {
                    clientX: event.clientX,
                    clientY: event.clientY,
                  });
                }
              }}
              onMouseMove={event => {
                if (segment.tooltip) {
                  showSegmentTooltip(segment.tooltip, {
                    clientX: event.clientX,
                    clientY: event.clientY,
                  });
                }
              }}
              onMouseLeave={() => setTooltip(null)}
              onFocus={event => {
                if (!segment.tooltip) return;
                const rect = event.currentTarget.getBoundingClientRect();
                showSegmentTooltip(segment.tooltip, {
                  clientX: rect.left + rect.width / 2,
                  clientY: rect.top,
                });
              }}
              onBlur={() => setTooltip(null)}
            >
              {showLabels && segment.label && (
                <SegmentValueLabel
                  label={segment.label}
                  segmentColor={segment.color}
                  testId={labelTestId}
                  className={segment.labelClassName}
                  autoContrast={segment.autoLabelContrast}
                />
              )}
            </div>
          ))}
        </div>
      </div>
      <PointerTooltipPortal hover={tooltip?.pointer ?? null}>
        {tooltip?.content}
      </PointerTooltipPortal>
    </>
  );
}
