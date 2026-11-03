// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DynamicAttribute } from '@quent/utils';
export type { TimelineSeries, TimelineSeriesEntry } from '../ui/charts/timelineTypes';
export {
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from '../ui/charts/timelineTypes';

export interface TimelineMark {
  label: string;
  stateName: string;
  color: string;
  xStart: number;
  xEnd: number;
  isDimmed?: boolean;
  operatorName?: string;
  attributes?: DynamicAttribute[];
  derivedAttributes?: DynamicAttribute[];
}

export const CHART_GROUP = 'timeline-sync-group';
