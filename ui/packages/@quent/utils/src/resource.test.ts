// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { resolveResourceFsmType, resourceFsmChoices } from './resource';
import type { ResourceGroupTypeDecl, ResourceTypeDecl } from './types';

const declaration: ResourceTypeDecl = {
  name: 'thread',
  capacities: [],
  display_order: [{ Type: 'task' }, { Type: 'worker' }],
};

describe('resolveResourceFsmType', () => {
  it('selects the first FSM type in declaration order', () => {
    expect(resolveResourceFsmType(declaration)).toBe('task');
    expect(
      resolveResourceFsmType({
        ...declaration,
        display_order: [{ Type: 'worker' }, { Type: 'task' }],
      })
    ).toBe('worker');
  });

  it('preserves explicit selections, including all FSMs', () => {
    expect(resolveResourceFsmType(declaration, 'worker')).toBe('worker');
    expect(resolveResourceFsmType(declaration, null)).toBeNull();
  });

  it('selects all FSMs when no types are declared', () => {
    expect(resolveResourceFsmType({ ...declaration, display_order: [] })).toBeNull();
    expect(resolveResourceFsmType(undefined)).toBeNull();
  });

  it('uses group declaration order for FSM types available on the selected resource', () => {
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: [{ Type: 'unavailable' }, { Type: 'worker' }, { Type: 'task' }],
    };
    expect(resolveResourceFsmType(declaration, undefined, group)).toBe('worker');
    expect(resolveResourceFsmType(declaration, null, group)).toBeNull();
    expect(
      resolveResourceFsmType(declaration, undefined, {
        ...group,
        display_order: [],
      })
    ).toBe('task');
  });
});

describe('analyzer-declared All choice', () => {
  it('selects All when it is first, and a specific type when All follows it', () => {
    expect(
      resolveResourceFsmType({
        ...declaration,
        display_order: ['All', { Type: 'task' }, { Type: 'worker' }],
      })
    ).toBeNull();
    expect(
      resolveResourceFsmType({
        ...declaration,
        display_order: [{ Type: 'task' }, 'All', { Type: 'worker' }],
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
    expect(resolveResourceFsmType(declaration, undefined, group)).toBeNull();
    expect(resolveResourceFsmType(declaration, 'task', group)).toBe('task');
  });
});

describe('single FSM default', () => {
  const single: ResourceTypeDecl = { ...declaration, display_order: [{ Type: 'task' }] };

  it('selects the only FSM type even when All is listed first', () => {
    expect(resolveResourceFsmType(single)).toBe('task');
    expect(resolveResourceFsmType({ ...single, display_order: ['All', { Type: 'task' }] })).toBe(
      'task'
    );
  });

  it('still preserves an explicit All selection', () => {
    expect(
      resolveResourceFsmType({ ...single, display_order: ['All', { Type: 'task' }] }, null)
    ).toBeNull();
  });

  it('still defaults to All when several FSM types are available', () => {
    expect(
      resolveResourceFsmType({
        ...single,
        display_order: ['All', { Type: 'task' }, { Type: 'worker' }],
      })
    ).toBeNull();
  });

  it('counts only FSM types available to the resource when a group declaration is given', () => {
    const group: ResourceGroupTypeDecl = {
      name: 'worker',
      contains_resource_types: ['thread'],
      display_order: ['All', { Type: 'unavailable' }, { Type: 'task' }],
    };
    expect(resolveResourceFsmType(single, undefined, group)).toBe('task');
  });
});
