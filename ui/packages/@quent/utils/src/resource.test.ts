// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vite-plus/test';
import { resolveResourceFsmType, resourceFsmChoices } from './resource';
import type { ResourceGroupTypeDecl, ResourceTypeDecl } from './types';

const declaration: ResourceTypeDecl = {
  name: 'thread',
  capacities: [],
  display_order: [{ Type: 'task' }, { Type: 'worker' }],
};

describe('resolveResourceFsmType', () => {
  it('selects the first FSM type in declaration order', () => {
    expect(resolveResourceFsmType({ resourceType: declaration })).toBe('task');
    expect(
      resolveResourceFsmType({
        resourceType: { ...declaration, display_order: [{ Type: 'worker' }, { Type: 'task' }] },
      })
    ).toBe('worker');
  });

  it('preserves explicit selections, including all FSMs', () => {
    expect(resolveResourceFsmType({ resourceType: declaration, selectedFsmType: 'worker' })).toBe(
      'worker'
    );
    expect(resolveResourceFsmType({ resourceType: declaration, selectedFsmType: null })).toBeNull();
  });

  it('selects all FSMs when no types are declared', () => {
    expect(
      resolveResourceFsmType({ resourceType: { ...declaration, display_order: [] } })
    ).toBeNull();
    expect(resolveResourceFsmType({})).toBeNull();
  });

  it('uses group declaration order for FSM types available on the selected resource', () => {
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: [{ Type: 'unavailable' }, { Type: 'worker' }, { Type: 'task' }],
    };
    expect(resolveResourceFsmType({ resourceType: declaration, resourceGroupType: group })).toBe(
      'worker'
    );
    expect(
      resolveResourceFsmType({
        resourceType: declaration,
        selectedFsmType: null,
        resourceGroupType: group,
      })
    ).toBeNull();
    expect(
      resolveResourceFsmType({
        resourceType: declaration,
        resourceGroupType: { ...group, display_order: [] },
      })
    ).toBe('task');
  });
});

describe('analyzer-declared All choice', () => {
  it('selects All when it is first, and a specific type when All follows it', () => {
    expect(
      resolveResourceFsmType({
        resourceType: {
          ...declaration,
          display_order: ['All', { Type: 'task' }, { Type: 'worker' }],
        },
      })
    ).toBeNull();
    expect(
      resolveResourceFsmType({
        resourceType: {
          ...declaration,
          display_order: [{ Type: 'task' }, 'All', { Type: 'worker' }],
        },
      })
    ).toBe('task');
  });

  it('does not add All when it is omitted', () => {
    expect(resourceFsmChoices(declaration)).toEqual([{ Type: 'task' }, { Type: 'worker' }]);
  });

  it('uses the group declaration to expose and default to All', () => {
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: ['All', { Type: 'worker' }, { Type: 'task' }],
    };
    expect(resourceFsmChoices(declaration, group)).toEqual([
      'All',
      { Type: 'worker' },
      { Type: 'task' },
    ]);
    expect(
      resolveResourceFsmType({ resourceType: declaration, resourceGroupType: group })
    ).toBeNull();
    expect(
      resolveResourceFsmType({
        resourceType: declaration,
        selectedFsmType: 'task',
        resourceGroupType: group,
      })
    ).toBe('task');
  });
});

describe('single FSM default', () => {
  const single: ResourceTypeDecl = { ...declaration, display_order: [{ Type: 'task' }] };

  it('selects the only FSM type even when All is listed first', () => {
    expect(resolveResourceFsmType({ resourceType: single })).toBe('task');
    expect(
      resolveResourceFsmType({
        resourceType: { ...single, display_order: ['All', { Type: 'task' }] },
      })
    ).toBe('task');
  });

  it('does not offer All when only one FSM type is available', () => {
    expect(resourceFsmChoices({ ...single, display_order: ['All', { Type: 'task' }] })).toEqual([
      { Type: 'task' },
    ]);
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: ['All', { Type: 'unavailable' }, { Type: 'task' }],
    };
    expect(resourceFsmChoices(single, group)).toEqual([{ Type: 'task' }]);
  });

  it('still offers All when several FSM types are available', () => {
    expect(
      resourceFsmChoices({
        ...single,
        display_order: ['All', { Type: 'task' }, { Type: 'worker' }],
      })
    ).toEqual(['All', { Type: 'task' }, { Type: 'worker' }]);
  });

  it('still preserves an explicit All selection', () => {
    expect(
      resolveResourceFsmType({
        resourceType: { ...single, display_order: ['All', { Type: 'task' }] },
        selectedFsmType: null,
      })
    ).toBeNull();
  });

  it('still defaults to All when several FSM types are available', () => {
    expect(
      resolveResourceFsmType({
        resourceType: {
          ...single,
          display_order: ['All', { Type: 'task' }, { Type: 'worker' }],
        },
      })
    ).toBeNull();
  });

  it('counts only FSM types available to the resource when a group declaration is given', () => {
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: ['All', { Type: 'unavailable' }, { Type: 'task' }],
    };
    expect(resolveResourceFsmType({ resourceType: single, resourceGroupType: group })).toBe('task');
  });
});
