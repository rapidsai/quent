// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TreeTableItem } from '@quent/components';
import { EntityTypeKey, type EntityRef, type QueryBundle } from '@quent/utils';
import { createLongEntitiesTimelineSubRow } from './LongEntitiesTimelineSubRow';

function item(id: string, type: string, children?: TreeTableItem[]): TreeTableItem {
  return { id, type, entity: {} as TreeTableItem['entity'], children };
}

const queryBundle = {} as unknown as QueryBundle<EntityRef>;

describe('createLongEntitiesTimelineSubRow', () => {
  it('injects an Entities sub-row after every resource by default', () => {
    const subRow = createLongEntitiesTimelineSubRow({
      engineId: 'engine-1',
      queryBundle,
      isDark: false,
    });
    const root = item('root', 'group', [
      item('res-1', EntityTypeKey.Resource),
      item('res-2', EntityTypeKey.Resource),
    ]);

    const tree = subRow.injectRows(root);

    expect(tree.children?.map(child => child.id)).toEqual([
      'res-1',
      '__long_entities__res-1',
      'res-2',
      '__long_entities__res-2',
    ]);
  });
});
