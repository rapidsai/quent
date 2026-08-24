// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { OperatorFilter, TimelineRequest } from '@quent/utils';
import { timelineCacheKey } from '../atoms/timeline';
import { buildMergedBulkEntries } from './useBulkTimelineFetch';

function makeResourceRequest(resourceId: string): TimelineRequest<OperatorFilter> {
  return {
    Resource: {
      resource_id: resourceId,
      long_entities_threshold_s: null,
      entity_filter: { entity_type_name: null },
      application: { operator_ids: [] },
      config: { window_start_s: 0, window_end_s: 1, num_bins: 10 } as never,
    },
  };
}

describe('buildMergedBulkEntries', () => {
  it('uses every selected operator in the request and entry key', () => {
    const result = buildMergedBulkEntries({ resource: makeResourceRequest('resource') }, [
      'op-2',
      'op-1',
    ]);
    const operatorEntryId = 'resource:ops:["op-1","op-2"]';
    const operatorEntry = result.entries[operatorEntryId];

    expect(operatorEntry).toBeDefined();
    expect(
      operatorEntry && 'Resource' in operatorEntry
        ? operatorEntry.Resource.application.operator_ids
        : []
    ).toEqual(['op-1', 'op-2']);
    expect(result.idToMeta.get(operatorEntryId)?.operatorIds).toEqual(['op-1', 'op-2']);
  });

  it('builds the same request key for the same operator set in any order', () => {
    const baseEntries = { resource: makeResourceRequest('resource') };

    const first = buildMergedBulkEntries(baseEntries, ['op-2', 'op-1']);
    const second = buildMergedBulkEntries(baseEntries, ['op-1', 'op-2', 'op-1']);

    expect(first.requestKey).toBe(second.requestKey);
    expect(Object.keys(first.entries)).toEqual(Object.keys(second.entries));
  });
});

describe('timelineCacheKey', () => {
  it('treats operator IDs as an order-independent set', () => {
    const base = { resourceId: 'resource', resourceTypeName: '', fsmTypeName: null };

    expect(timelineCacheKey({ ...base, operatorIds: ['op-2', 'op-1'] })).toBe(
      timelineCacheKey({ ...base, operatorIds: ['op-1', 'op-2', 'op-1'] })
    );
  });
});
