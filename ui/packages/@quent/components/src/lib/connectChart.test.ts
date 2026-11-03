// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { describe, expect, it, vi } from 'vitest';
import { CHART_GROUP } from '../timeline/types';
import { connectChart } from './connectChart';

const { connectGenericChart } = vi.hoisted(() => ({ connectGenericChart: vi.fn() }));
vi.mock('../ui/charts/connectChart', () => ({ connectChart: connectGenericChart }));

describe('connectChart compatibility wrapper', () => {
  it('preserves the default timeline group', () => {
    const instance = {} as EChartsInstance;
    connectChart(instance);
    expect(connectGenericChart).toHaveBeenCalledWith(instance, CHART_GROUP, true, null);
  });
});
