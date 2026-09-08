// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from 'vitest';

const { MockEChartsReactCore } = vi.hoisted(() => ({
  MockEChartsReactCore: () => null,
}));

vi.mock('echarts-for-react/lib/core', () => ({
  default: { default: MockEChartsReactCore },
}));

import { EChartsReactCore } from './echartsReactCore';

describe('EChartsReactCore', () => {
  it('unwraps the Vite 8 CommonJS module object', () => {
    expect(EChartsReactCore).toBe(MockEChartsReactCore);
  });
});
