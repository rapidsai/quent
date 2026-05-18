// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { TreeTableItem } from '../resource-tree/types';
import { collectResourceTypesFromTree } from './resource.utils';

function makeResourceItem(id: string, typeName: string, children?: TreeTableItem[]): TreeTableItem {
  return {
    id,
    type: 'resource',
    entity: { id, instance_name: id, type_name: typeName, parent_group_id: 'g1' },
    children,
  };
}

function makeGroupItem(id: string, typeName: string, children?: TreeTableItem[]): TreeTableItem {
  return {
    id,
    type: 'resource-group',
    entity: { id, instance_name: id, type_name: typeName, parent_group_id: null },
    children,
  };
}

describe('collectResourceTypesFromTree', () => {
  it('returns [] for an empty array', () => {
    expect(collectResourceTypesFromTree([])).toEqual([]);
  });

  it('returns the type_name of a leaf item', () => {
    const items = [makeResourceItem('r1', 'GPU')];
    expect(collectResourceTypesFromTree(items)).toEqual(['GPU']);
  });

  it('deduplicates repeated type names', () => {
    const items = [
      makeResourceItem('r1', 'GPU'),
      makeResourceItem('r2', 'GPU'),
      makeResourceItem('r3', 'CPU'),
    ];
    const result = collectResourceTypesFromTree(items);
    expect(result).toHaveLength(2);
    expect(result).toContain('GPU');
    expect(result).toContain('CPU');
  });

  it('collects type names from nested leaf nodes', () => {
    const items = [
      makeGroupItem('g1', 'Engine', [makeResourceItem('r1', 'GPU'), makeResourceItem('r2', 'CPU')]),
    ];
    const result = collectResourceTypesFromTree(items);
    expect(result).toContain('GPU');
    expect(result).toContain('CPU');
    expect(result).toHaveLength(2);
  });

  it('does not collect type_name from a non-leaf group node', () => {
    const items = [makeGroupItem('g1', 'Engine', [makeResourceItem('r1', 'GPU')])];
    const result = collectResourceTypesFromTree(items);
    expect(result).not.toContain('Engine');
    expect(result).toEqual(['GPU']);
  });

  it('collects type names from deeply nested leaves', () => {
    const items = [
      makeGroupItem('g1', 'Top', [makeGroupItem('g2', 'Mid', [makeResourceItem('r1', 'SSD')])]),
    ];
    expect(collectResourceTypesFromTree(items)).toEqual(['SSD']);
  });

  it('skips items whose entity has no type_name', () => {
    const item: TreeTableItem = {
      id: 'q1',
      type: 'query',
      entity: { id: 'q1', instance_name: 'query1' } as never,
    };
    expect(collectResourceTypesFromTree([item])).toEqual([]);
  });

  it('collects from multiple top-level items', () => {
    const items = [makeResourceItem('r1', 'GPU'), makeResourceItem('r2', 'NIC')];
    const result = collectResourceTypesFromTree(items);
    expect(result).toContain('GPU');
    expect(result).toContain('NIC');
    expect(result).toHaveLength(2);
  });
});
