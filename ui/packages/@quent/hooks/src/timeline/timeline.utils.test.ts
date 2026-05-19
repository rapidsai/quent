// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { TaskFilter, TimelineRequest } from '@quent/utils';
import {
  getResourceTypeName,
  getFsmTypeName,
  bulkEntryId,
  setOperatorOnEntry,
} from './timeline.utils';

function makeResourceRequest(
  resourceId: string,
  entityTypeName: string | null = null,
  operatorId: string | null = null
): TimelineRequest<TaskFilter> {
  return {
    Resource: {
      resource_id: resourceId,
      long_entities_threshold_s: null,
      entity_filter: { entity_type_name: entityTypeName },
      application: { operator_id: operatorId },
      config: { window_start_s: 0, window_end_s: 1, num_bins: 10 } as never,
    },
  };
}

function makeGroupRequest(
  groupId: string,
  resourceTypeName: string,
  entityTypeName: string | null = null,
  operatorId: string | null = null
): TimelineRequest<TaskFilter> {
  return {
    ResourceGroup: {
      resource_group_id: groupId,
      resource_type_name: resourceTypeName,
      long_entities_threshold_s: null,
      entity_filter: { entity_type_name: entityTypeName },
      app_params: { operator_id: operatorId },
      config: { window_start_s: 0, window_end_s: 1, num_bins: 10 } as never,
    },
  };
}

describe('getResourceTypeName', () => {
  it('returns empty string for undefined', () => {
    expect(getResourceTypeName(undefined)).toBe('');
  });

  it('returns empty string for a Resource variant', () => {
    expect(getResourceTypeName(makeResourceRequest('res-1'))).toBe('');
  });

  it('returns resource_type_name for a ResourceGroup variant', () => {
    expect(getResourceTypeName(makeGroupRequest('g1', 'GPU'))).toBe('GPU');
  });

  it('returns an empty string when resource_type_name is empty', () => {
    expect(getResourceTypeName(makeGroupRequest('g1', ''))).toBe('');
  });
});

describe('getFsmTypeName', () => {
  it('returns entity_type_name from a Resource variant', () => {
    expect(getFsmTypeName(makeResourceRequest('r1', 'QueryOperator'))).toBe('QueryOperator');
  });

  it('returns null entity_type_name from a Resource variant', () => {
    expect(getFsmTypeName(makeResourceRequest('r1', null))).toBeNull();
  });

  it('returns entity_type_name from a ResourceGroup variant', () => {
    expect(getFsmTypeName(makeGroupRequest('g1', 'GPU', 'Worker'))).toBe('Worker');
  });

  it('returns null entity_type_name from a ResourceGroup variant', () => {
    expect(getFsmTypeName(makeGroupRequest('g1', 'GPU', null))).toBeNull();
  });
});

describe('bulkEntryId', () => {
  it('returns "<resourceId>:base" when operatorId is omitted', () => {
    expect(bulkEntryId('res-1')).toBe('res-1:base');
  });

  it('returns "<resourceId>:base" when operatorId is undefined', () => {
    expect(bulkEntryId('res-1', undefined)).toBe('res-1:base');
  });

  it('returns "<resourceId>:base" when operatorId is null', () => {
    expect(bulkEntryId('res-1', null)).toBe('res-1:base');
  });

  it('returns "<resourceId>:op:<operatorId>" when operatorId is provided', () => {
    expect(bulkEntryId('res-1', 'op-42')).toBe('res-1:op:op-42');
  });
});

describe('setOperatorOnEntry', () => {
  it('sets operator_id on a Resource variant', () => {
    const entry = makeResourceRequest('r1', null, null);
    const result = setOperatorOnEntry(entry, 'op-99');
    expect('Resource' in result).toBe(true);
    expect(
      (result as { Resource: { application: TaskFilter } }).Resource.application.operator_id
    ).toBe('op-99');
  });

  it('preserves other Resource fields when setting operator_id', () => {
    const entry = makeResourceRequest('r1', 'Fsm', null);
    const result = setOperatorOnEntry(entry, 'op-1') as {
      Resource: (typeof entry)['Resource' & keyof typeof entry];
    };
    expect(result.Resource.resource_id).toBe('r1');
    expect(result.Resource.entity_filter.entity_type_name).toBe('Fsm');
  });

  it('does not mutate the original Resource entry', () => {
    const entry = makeResourceRequest('r1');
    const original = (entry as { Resource: { application: TaskFilter } }).Resource.application
      .operator_id;
    setOperatorOnEntry(entry, 'op-new');
    expect(
      (entry as { Resource: { application: TaskFilter } }).Resource.application.operator_id
    ).toBe(original);
  });

  it('sets operator_id on a ResourceGroup variant', () => {
    const entry = makeGroupRequest('g1', 'GPU', null, null);
    const result = setOperatorOnEntry(entry, 'op-7');
    expect('ResourceGroup' in result).toBe(true);
    expect(
      (result as { ResourceGroup: { app_params: TaskFilter } }).ResourceGroup.app_params.operator_id
    ).toBe('op-7');
  });

  it('preserves other ResourceGroup fields when setting operator_id', () => {
    const entry = makeGroupRequest('g1', 'GPU', 'Worker', null);
    const result = setOperatorOnEntry(entry, 'op-2') as {
      ResourceGroup: (typeof entry)['ResourceGroup' & keyof typeof entry];
    };
    expect(result.ResourceGroup.resource_group_id).toBe('g1');
    expect(result.ResourceGroup.resource_type_name).toBe('GPU');
    expect(result.ResourceGroup.entity_filter.entity_type_name).toBe('Worker');
  });

  it('does not mutate the original ResourceGroup entry', () => {
    const entry = makeGroupRequest('g1', 'GPU');
    const original = (entry as { ResourceGroup: { app_params: TaskFilter } }).ResourceGroup
      .app_params.operator_id;
    setOperatorOnEntry(entry, 'op-new');
    expect(
      (entry as { ResourceGroup: { app_params: TaskFilter } }).ResourceGroup.app_params.operator_id
    ).toBe(original);
  });
});
