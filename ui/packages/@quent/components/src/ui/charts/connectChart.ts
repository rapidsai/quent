// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { connect } from './echarts';

export interface ChartZoomPercentRange {
  start: number;
  end: number;
}

export function connectChart(
  instance: EChartsInstance,
  chartGroup: string,
  activateBrushSelect = true,
  zoomPct: ChartZoomPercentRange | null = null
) {
  if (zoomPct) {
    instance.dispatchAction({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: zoomPct.start,
      end: zoomPct.end,
    });
  }

  if (activateBrushSelect) {
    instance.dispatchAction({
      type: 'takeGlobalCursor',
      key: 'dataZoomSelect',
      dataZoomSelectActive: true,
    });
  }

  instance.group = chartGroup;
  connect(chartGroup);
}
