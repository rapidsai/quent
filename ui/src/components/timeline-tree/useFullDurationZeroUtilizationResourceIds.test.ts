// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { BulkTimelinesResponse, QueryEntities } from '@quent/utils';
import { renderHookWithQuery } from '@/test/test-utils';
import { useFullDurationZeroUtilizationResourceIds } from './useFullDurationZeroUtilizationResourceIds';

const fetchBulkTimelines = vi.fn();

vi.mock('@quent/client', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/client')>();
  return {
    ...actual,
    fetchBulkTimelines: (...args: unknown[]) => fetchBulkTimelines(...args),
    bulkTimelineQueryOptions: (
      params: { engineId: string; request: unknown },
      options?: { staleTime?: number }
    ) => ({
      queryKey: ['bulkTimelines', params.engineId, params.request],
      queryFn: () => fetchBulkTimelines(params.engineId, params.request),
      staleTime: options?.staleTime,
    }),
  };
});

const entitiesFixture: Pick<QueryEntities, 'resources' | 'resource_types'> = {
  resources: {
    'zero-resource': {
      id: 'zero-resource',
      type_name: 'GPU',
      instance_name: 'GPU 0',
      parent_group_id: 'engine-1',
    },
    'busy-resource': {
      id: 'busy-resource',
      type_name: 'GPU',
      instance_name: 'GPU 1',
      parent_group_id: 'engine-1',
    },
  },
  resource_types: {
    GPU: { name: 'GPU', capacities: [], used_by: ['Worker'] },
  },
};
const entities = entitiesFixture as QueryEntities;

function response(overrides: Partial<BulkTimelinesResponse['entries']>): BulkTimelinesResponse {
  return { entries: overrides as BulkTimelinesResponse['entries'] };
}

describe('useFullDurationZeroUtilizationResourceIds', () => {
  it('includes only resources whose full-duration utilization is all zero', async () => {
    fetchBulkTimelines.mockResolvedValue(
      response({
        'zero-resource': {
          status: 'ok',
          message: '',
          config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
          data: {
            Binned: {
              config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
              capacities_values: { count: [0] },
              long_fsms: [],
            },
          },
        },
        'busy-resource': {
          status: 'ok',
          message: '',
          config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
          data: {
            Binned: {
              config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
              capacities_values: { count: [5] },
              long_fsms: [],
            },
          },
        },
      })
    );

    const { result } = renderHookWithQuery(() =>
      useFullDurationZeroUtilizationResourceIds('engine-1', 'query-1', 100, entities)
    );

    await waitFor(() => expect(result.current.has('zero-resource')).toBe(true));
    expect(result.current.has('busy-resource')).toBe(false);
  });

  it('excludes resources whose response entry errored', async () => {
    fetchBulkTimelines.mockResolvedValue(
      response({
        'zero-resource': { status: 'error', message: 'boom' },
        'busy-resource': {
          status: 'ok',
          message: '',
          config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
          data: {
            Binned: {
              config: { span: { start: 0, end: 100 }, bin_duration: 100, num_bins: 1n },
              capacities_values: { count: [5] },
              long_fsms: [],
            },
          },
        },
      })
    );

    const { result } = renderHookWithQuery(() =>
      useFullDurationZeroUtilizationResourceIds('engine-1', 'query-1', 100, entities)
    );

    await waitFor(() => expect(fetchBulkTimelines).toHaveBeenCalled());
    expect(result.current.size).toBe(0);
  });

  it('returns an empty set before the response arrives', () => {
    fetchBulkTimelines.mockReturnValue(new Promise(() => {}));

    const { result } = renderHookWithQuery(() =>
      useFullDurationZeroUtilizationResourceIds('engine-1', 'query-1', 100, entities)
    );

    expect(result.current.size).toBe(0);
  });

  it('requests a single bin spanning the full query duration for every resource', () => {
    fetchBulkTimelines.mockReturnValue(new Promise(() => {}));

    renderHookWithQuery(() =>
      useFullDurationZeroUtilizationResourceIds('engine-1', 'query-1', 100, entities)
    );

    expect(fetchBulkTimelines).toHaveBeenCalledWith('engine-1', {
      entries: {
        'zero-resource': {
          Resource: {
            resource_id: 'zero-resource',
            long_entities_threshold_s: null,
            entity_filter: { entity_type_name: 'Worker' },
            application: { operator_ids: [] },
            config: { num_bins: 1, start: 0, end: 100 },
          },
        },
        'busy-resource': {
          Resource: {
            resource_id: 'busy-resource',
            long_entities_threshold_s: null,
            entity_filter: { entity_type_name: 'Worker' },
            application: { operator_ids: [] },
            config: { num_bins: 1, start: 0, end: 100 },
          },
        },
      },
      app_params: { query_id: 'query-1' },
    });
  });
});
