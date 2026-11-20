// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { CHART_GROUP } from '../timeline/types';
import { connectChart as connectGenericChart } from '../ui/charts/connectChart';
import type { ChartZoomPercentRange } from '../ui/charts/connectChart';

/** Preserves the public timeline-group default over generic connect mechanics. */
export function connectChart(
  instance: EChartsInstance,
  chartGroup: string = CHART_GROUP,
  activateBrushSelect = true,
  zoomPct: ChartZoomPercentRange | null = null
) {
  connectGenericChart(instance, chartGroup, activateBrushSelect, zoomPct);
}
