// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import EChartsReactCoreImport from 'echarts-for-react/lib/core';

type EChartsReactCoreComponent = typeof EChartsReactCoreImport;
const cjsModule = EChartsReactCoreImport as unknown as {
  default?: EChartsReactCoreComponent;
};

// Vite 8 maps this CJS default import to module.exports for ESM importers.
export const EChartsReactCore = cjsModule.default ?? EChartsReactCoreImport;
