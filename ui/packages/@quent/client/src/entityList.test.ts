// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { keepPreviousData } from '@tanstack/react-query';
import type { EntityListResponse } from '@quent/utils';
import { entityListInfiniteQueryOptions, entityListQueryOptions } from './entityList';

describe('entityListQueryOptions', () => {
  it('copies selected operator IDs into the entity-list request', () => {
    const options = entityListQueryOptions({
      engineId: 'engine-1',
      queryId: 'query-1',
      window: { start: 0, end: 1 },
      operatorIds: ['operator-1'],
      minUsageSeconds: 0.15,
      maxItems: 20,
      page: 2,
    });

    expect(options.queryKey).toEqual([
      'entityList',
      'engine-1',
      expect.objectContaining({
        entry: expect.objectContaining({
          application: { operator_ids: ['operator-1'] },
          filter: expect.objectContaining({ min_usage_s: 0.15 }),
          page: { page: 2, max: 20 },
        }),
      }),
    ]);
    expect(options.placeholderData).toBe(keepPreviousData);
  });

  it('continues paging until all matching entities are loaded', () => {
    const options = entityListInfiniteQueryOptions({
      engineId: 'engine-1',
      queryId: 'query-1',
      window: { start: 0, end: 1 },
      maxItems: 1,
    });
    const item = {} as EntityListResponse['items'][number];
    const firstPage: EntityListResponse = { items: [item], total: 3 };
    const secondPage: EntityListResponse = { items: [item], total: 3 };

    expect(options.placeholderData).toBe(keepPreviousData);
    expect(options.getNextPageParam?.(secondPage, [firstPage, secondPage], 1, [0, 1])).toBe(2);
    expect(
      options.getNextPageParam?.({ items: [item], total: 2 }, [firstPage, secondPage], 1, [0, 1])
    ).toBeUndefined();
  });
});
