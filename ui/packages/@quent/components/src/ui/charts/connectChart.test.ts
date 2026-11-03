// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EChartsInstance } from 'echarts-for-react';
import { describe, expect, it, vi } from 'vitest';
import { connectChart } from './connectChart';

const { connect } = vi.hoisted(() => ({ connect: vi.fn() }));
vi.mock('./echarts', () => ({ connect }));

describe('connectChart', () => {
  it('applies zoom, activates brush selection, and joins the group', () => {
    const instance = { dispatchAction: vi.fn(), group: undefined } as unknown as EChartsInstance;

    connectChart(instance, 'test-group', true, { start: 10, end: 80 });

    expect(instance.dispatchAction).toHaveBeenNthCalledWith(1, {
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: 10,
      end: 80,
    });
    expect(instance.dispatchAction).toHaveBeenNthCalledWith(2, {
      type: 'takeGlobalCursor',
      key: 'dataZoomSelect',
      dataZoomSelectActive: true,
    });
    expect(instance.group).toBe('test-group');
    expect(connect).toHaveBeenCalledWith('test-group');
  });
});
