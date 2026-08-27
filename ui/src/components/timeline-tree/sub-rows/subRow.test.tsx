// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TreeTableItem } from '@quent/components';
import { EntityTypeKey } from '@quent/utils';
import { createSyntheticSubRow, mapTreeItems } from './subRow';

function item(id: string, type: string, children?: TreeTableItem[]): TreeTableItem {
  return { id, type, entity: {} as TreeTableItem['entity'], children };
}

describe('mapTreeItems', () => {
  it('appends a sibling after matching children', () => {
    const root = item('root', 'group', [item('res-1', EntityTypeKey.Resource)]);
    const next = mapTreeItems(root, (_self, children) => {
      const rewritten: TreeTableItem[] = [];
      for (const child of children) {
        rewritten.push(child);
        if (child.type === EntityTypeKey.Resource) {
          rewritten.push(createSyntheticSubRow(`${child.id}-sub`, 'sub'));
        }
      }
      return rewritten;
    });
    expect(next.children?.map(child => child.id)).toEqual(['res-1', 'res-1-sub']);
  });

  it('prepends a child on matching nodes, including leaves', () => {
    const root = item('root', 'group', [item('worker-1', EntityTypeKey.Resource)]);
    const next = mapTreeItems(root, (self, children) =>
      self.id === 'worker-1'
        ? [createSyntheticSubRow(`${self.id}-ops`, 'ops'), ...children]
        : children
    );
    expect(next.children?.[0]?.children?.map(child => child.id)).toEqual(['worker-1-ops']);
  });
});
