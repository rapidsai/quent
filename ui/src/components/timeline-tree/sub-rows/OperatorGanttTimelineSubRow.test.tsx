// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { createOperatorGanttTimelineSubRow } from './OperatorGanttTimelineSubRow';

const queryBundle = {
  duration_s: 1,
  plan_tree: { id: 'plan-1', worker: 'worker-1', children: [] },
  entities: { operators: {} },
} as unknown as QueryBundle<EntityRef>;

describe('createOperatorGanttTimelineSubRow', () => {
  const subRow = createOperatorGanttTimelineSubRow({ queryBundle, isDark: false });

  it('is labeled Operators', () => {
    expect(subRow.label).toBe('Operators');
  });
});
