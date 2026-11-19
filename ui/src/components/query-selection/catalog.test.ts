// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vite-plus/test';
import {
  catalogAttributeKeys,
  filterCatalogItems,
  groupCatalogItems,
  sortQueriesByMostRecent,
  type CatalogEntity,
} from './catalog';
import type { Query } from '@quent/utils';

const items: CatalogEntity[] = [
  {
    id: 'query-1',
    instance_name: 'Scan orders',
    custom_attributes: [
      { key: 'workload', value: 'nightly' },
      { key: 'rows', value: 42 },
    ],
  },
  {
    id: 'query-2',
    instance_name: 'Aggregate sales',
    custom_attributes: [{ key: 'workload', value: 'interactive' }],
  },
  {
    id: 'query-3',
    instance_name: null,
    custom_attributes: [],
  },
];

describe('catalog helpers', () => {
  it('derives attribute columns in first-seen order', () => {
    expect(catalogAttributeKeys(items)).toEqual(['workload', 'rows']);
  });

  it('searches names, ids, keys, and formatted values', () => {
    expect(filterCatalogItems(items, 'orders').map(item => item.id)).toEqual(['query-1']);
    expect(filterCatalogItems(items, 'interactive').map(item => item.id)).toEqual(['query-2']);
    expect(filterCatalogItems(items, 'rows').map(item => item.id)).toEqual(['query-1']);
    expect(filterCatalogItems(items, 'query-3').map(item => item.id)).toEqual(['query-3']);
  });

  it('groups missing values under Other while preserving row order', () => {
    expect(groupCatalogItems(items, 'workload')).toEqual([
      { id: 'value:nightly', label: 'nightly', items: [items[0]] },
      { id: 'value:interactive', label: 'interactive', items: [items[1]] },
      { id: 'missing', label: 'Other', items: [items[2]] },
    ]);
  });

  it('keeps missing values separate from a value matching the old sentinel', () => {
    const collisionItems: CatalogEntity[] = [
      {
        id: 'query-present',
        instance_name: null,
        custom_attributes: [{ key: 'workload', value: '__other__' }],
      },
      {
        id: 'query-missing',
        instance_name: null,
        custom_attributes: [],
      },
    ];

    expect(groupCatalogItems(collisionItems, 'workload')).toEqual([
      { id: 'value:__other__', label: '__other__', items: [collisionItems[0]] },
      { id: 'missing', label: 'Other', items: [collisionItems[1]] },
    ]);
  });

  it('sorts queries by newest start time with deterministic fallbacks', () => {
    const queries: Query[] = [
      {
        id: 'query-missing',
        instance_name: null,
        custom_attributes: [],
        start_unix_ns: null,
        planning_s: null,
        executing_s: null,
        completed_s: null,
      },
      {
        id: 'query-b',
        instance_name: 'Beta',
        custom_attributes: [],
        start_unix_ns: 20n,
        planning_s: null,
        executing_s: null,
        completed_s: null,
      },
      {
        id: 'query-a',
        instance_name: 'Alpha',
        custom_attributes: [],
        start_unix_ns: 20n,
        planning_s: null,
        executing_s: null,
        completed_s: null,
      },
      {
        id: 'query-old',
        instance_name: 'Old',
        custom_attributes: [],
        start_unix_ns: 10n,
        planning_s: null,
        executing_s: null,
        completed_s: null,
      },
    ];

    expect(sortQueriesByMostRecent(queries).map(query => query.id)).toEqual([
      'query-a',
      'query-b',
      'query-old',
      'query-missing',
    ]);
    expect(queries[0]?.id).toBe('query-missing');
  });
});
