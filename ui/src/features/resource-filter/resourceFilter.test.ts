// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TreeTableItem } from '@quent/components';
import { EntityTypeKey, type QueryEntities } from '@quent/utils';
import { EMPTY_RESOURCE_FILTER, filterResourceTree, type ResourceFilter } from './resourceFilter';

function resource(id: string, name: string, typeName: string): TreeTableItem {
  return {
    id,
    type: EntityTypeKey.Resource,
    entity: { id, instance_name: name, type_name: typeName, parent_group_id: 'workers' },
    children: [],
  };
}

function group(
  id: string,
  name: string,
  children: TreeTableItem[],
  typeName = 'ResourceGroup'
): TreeTableItem {
  return {
    id,
    type: EntityTypeKey.ResourceGroup,
    entity: { id, instance_name: name, type_name: typeName, parent_group_id: null },
    children,
  };
}

const gpu0 = resource('gpu-0', 'GPU 0', 'gpu');
const gpu1 = resource('gpu-1', 'GPU 1', 'gpu');
const cpu0 = resource('cpu-0', 'CPU 0', 'cpu');
const worker = group('worker-a', 'Worker A', [gpu0, gpu1]);
const root = group('query', 'Query resources', [worker, cpu0]);
const entities = {
  resource_types: {
    gpu: { name: 'gpu', used_by: ['task', 'transfer'], capacities: [] },
    cpu: { name: 'cpu', used_by: ['task'], capacities: [] },
  },
} satisfies Pick<QueryEntities, 'resource_types'>;

function filter(overrides: Partial<ResourceFilter>) {
  return filterResourceTree(root, entities, { ...EMPTY_RESOURCE_FILTER, ...overrides });
}

describe('filterResourceTree', () => {
  it('matches names and promotes matches through nonmatching ancestors', () => {
    const result = filter({ search: 'GPU 1' });

    expect(result.filteredItems.map(item => item.id)).toEqual(['gpu-1']);
    expect(result.directMatchIds).toEqual(new Set(['gpu-1']));
  });

  it('matches resource IDs', () => {
    const result = filter({ search: 'gpu-0' });

    expect(result.filteredItems.map(item => item.id)).toEqual(['gpu-0']);
    expect(result.directMatchIds).toEqual(new Set(['gpu-0']));
  });

  it('matches comma-delimited search terms as alternatives', () => {
    const result = filter({ search: 'gpu-0, cpu-0' });

    expect(result.filteredItems.map(item => item.id)).toEqual(['gpu-0', 'cpu-0']);
    expect(result.directMatchIds).toEqual(new Set(['gpu-0', 'cpu-0']));
  });

  it('searches displayed type labels', () => {
    const result = filter({ search: 'ResourceGroup' });

    expect(result.filteredItems.map(item => item.id)).toEqual(['query']);
    expect(result.filteredItems[0]?.children?.map(item => item.id)).toEqual(['worker-a']);
    expect(result.matchCount).toBe(2);
  });

  it('matches selected resource types as alternatives', () => {
    const result = filter({ resourceTypes: ['gpu', 'cpu'] });

    expect(result.directMatchIds).toEqual(new Set(['gpu-0', 'gpu-1', 'cpu-0']));
    expect(result.filteredItems.map(item => item.id)).toEqual(['gpu-0', 'gpu-1', 'cpu-0']);
  });

  it('matches selected FSM declarations as alternatives', () => {
    const result = filter({ fsmTypes: ['transfer', 'task'] });

    expect(result.directMatchIds).toEqual(new Set(['gpu-0', 'gpu-1', 'cpu-0']));
    expect(result.filteredItems.map(item => item.id)).toEqual(['gpu-0', 'gpu-1', 'cpu-0']);
  });

  it('ANDs the search and specific filters', () => {
    const result = filter({ search: 'GPU 0', fsmTypes: ['task'] });

    expect(result.directMatchIds).toEqual(new Set(['gpu-0']));
  });

  it('hides nonmatching descendants of a matched group', () => {
    const result = filter({ search: 'Worker A' });

    expect(result.filteredItems).toEqual([{ ...worker, children: [] }]);
  });

  it('returns no tree when nothing matches', () => {
    const result = filter({ search: 'missing' });

    expect(result.filteredItems).toEqual([]);
    expect(result.matchCount).toBe(0);
  });

  it('returns the original tree when no filters are active', () => {
    expect(filterResourceTree(root, entities, EMPTY_RESOURCE_FILTER).filteredItems).toEqual([root]);
    expect(filter({ showOthers: true }).isActive).toBe(false);
  });
});
