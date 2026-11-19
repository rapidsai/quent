// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TreeTableItem } from '@quent/components';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { createOperatorGanttTimelineSubRow } from './OperatorGanttTimelineSubRow';

function item(id: string, type: string, children?: TreeTableItem[]): TreeTableItem {
  return { id, type, entity: {} as TreeTableItem['entity'], children };
}

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

  it('injects an Operators disclosure beneath its worker group', () => {
    const tree = subRow.injectRows(
      item('engine', 'group', [
        item('worker-1', 'group', [item('resource-1', 'resource')]),
        item('worker-2', 'group'),
      ])
    );

    expect(tree.children?.[0]?.children?.map(child => child.id)).toEqual([
      '__operator_timeline_disclosure__worker-1',
      'resource-1',
    ]);
    expect(tree.children?.[0]?.children?.[0]?.type).toBe('operator-timeline-disclosure');
    expect(tree.children?.[0]?.children?.[0]?.children?.[0]?.id).toBe(
      '__operator_timeline__worker-1'
    );
    expect(tree.children?.[1]?.children).toBeUndefined();
  });
});
