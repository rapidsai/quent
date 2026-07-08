// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Attribute } from '@quent/utils';

export type TimelineSeriesEntry = {
  binDuration: number;
  formatter: (value: number, decimals?: number) => string;
  values: number[];
  color: string;
  isOverlay?: boolean;
  /** When true, this series is dimmed to make overlay series stand out. */
  isDimmed?: boolean;
};

export type TimelineSeries = Record<string, TimelineSeriesEntry>;

/** A single annotation mark on the timeline. */
export type TimelineMark = {
  label: string;
  stateName: string;
  color: string;
  xStart: number;
  xEnd: number;
  /** When true, this mark is dimmed (e.g. not part of the selected operator's long entities). */
  isDimmed?: boolean;
  /** Operator instance name when this mark belongs to a selected operator's long entities. */
  operatorName?: string;
  /** Attribute key-value pairs carried by the state this mark represents. */
  attributes?: Attribute[];
  /** The pipeline (fused operator chain) the task executes, resolved from the
   *  FSM's pipeline_uuid attribute. Attached to every mark of the FSM. */
  pipeline?: TimelineMarkPipeline;
};

/** Pipeline descriptor resolved from a task FSM's pipeline_uuid. */
export type TimelineMarkPipeline = {
  /** The fused operator chain, e.g. "GPU_SCAN(11) -> PROJECTION(6) -> …". */
  name: string;
  /** The operator type name, e.g. "Pipeline Id 0". */
  typeName?: string | null;
};

export const DEFAULT_TIMELINE_HEIGHT = 45;

// left/right spacing needs to be consistent across all timelines
// so axes line up. top/bottom spacing can be overridden, but defaults still
// provided here
export const TIMELINE_SPACING = {
  left: 0,
  right: 10,
  top: 2.5,
  bottom: 2.5,
};

// Timeline color constants live in timelineEchartsTheme (canvas-based, theme mirrored in JS).

// Shared axis animation settings for timeline charts.
export const TIMELINE_X_AXIS_ANIMATION = {
  animation: false,
  animationDuration: 50,
  animationDurationUpdate: 100,
  animationEasingUpdate: 'cubicOut',
} as const;
