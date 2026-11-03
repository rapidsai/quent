// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export interface TimelineSeriesEntry {
  binDuration: number;
  formatter: (value: number, decimals?: number) => string;
  values: number[];
  color: string;
  isOverlay?: boolean;
  isDimmed?: boolean;
}

export type TimelineSeries = Record<string, TimelineSeriesEntry>;

export interface TimelineMark {
  labelLines: string[];
  color: string;
  xStart: number;
  xEnd: number;
  isDimmed?: boolean;
}

export const DEFAULT_TIMELINE_HEIGHT = 45;

export const TIMELINE_SPACING = {
  left: 0,
  right: 10,
  top: 2.5,
  bottom: 2.5,
  outerBoundsMode: 'none' as const,
};

export const TIMELINE_X_AXIS_ANIMATION = {
  animation: false,
  animationDuration: 50,
  animationDurationUpdate: 100,
  animationEasingUpdate: 'cubicOut',
} as const;
