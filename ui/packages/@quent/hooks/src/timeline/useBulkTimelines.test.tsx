// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { PropsWithChildren } from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import type { OperatorFilter, QueryEntities, TimelineRequest } from '@quent/utils';
import { visibleEntriesAtom } from '../atoms/timeline';
import { useBulkTimelines, type TreeNode } from './useBulkTimelines';

vi.mock('@quent/client', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/client')>();
  return {
    ...actual,
    fetchBulkTimelines: vi.fn().mockResolvedValue({ entries: {} }),
  };
});

function request(id: string): TimelineRequest<OperatorFilter> {
  return {
    Resource: {
      resource_id: id,
      long_entities_threshold_s: null,
      entity_filter: { entity_type_name: null },
      application: { operator_ids: [] },
      config: { window_start_s: 0, window_end_s: 1, num_bins: 10 } as never,
    },
  } as TimelineRequest<OperatorFilter>;
}

const gpu: TreeNode = { id: 'gpu-0' };
const worker: TreeNode = { id: 'worker-1', children: [gpu] };
const root: TreeNode = { id: 'root', children: [worker] };

// Only walks into expanded rows, then adds extra items, like the real collectVisibleEntries.
function collectVisible(
  items: TreeNode[],
  expandedIds: Set<string>,
  extraItems: readonly TreeNode[] = []
): Record<string, TimelineRequest<OperatorFilter>> {
  const result: Record<string, TimelineRequest<OperatorFilter>> = {};
  const walk = (item: TreeNode) => {
    result[item.id] = request(item.id);
    if (item.children && expandedIds.has(item.id)) {
      item.children.forEach(walk);
    }
  };
  items.forEach(walk);
  extraItems.forEach(item => (result[item.id] ??= request(item.id)));
  return result;
}

function renderBulk(extraItems?: TreeNode[]) {
  const store = createStore();
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const wrapper = ({ children }: PropsWithChildren) => (
    <QueryClientProvider client={queryClient}>
      <Provider store={store}>{children}</Provider>
    </QueryClientProvider>
  );
  renderHook(
    () =>
      useBulkTimelines<TreeNode>({
        engineId: 'engine-1',
        queryId: 'query-1',
        rootItem: root,
        expandedIds: new Set(['root']),
        selectedTypes: new Map(),
        entities: {} as QueryEntities,
        collectVisibleEntriesFn: (items, _expanded, _types, _entities, _config, _filters, extra) =>
          collectVisible(items, new Set(['root']), extra),
        buildBulkParamsFn: item => request(item.id),
        findItemByIdFn: () => undefined,
        extraItems,
      }),
    { wrapper }
  );
  return store;
}

describe('useBulkTimelines extraItems', () => {
  it('requests only the expanded rows when there are no extra items', async () => {
    const store = renderBulk();

    await waitFor(() =>
      expect(Object.keys(store.get(visibleEntriesAtom)).length).toBeGreaterThan(0)
    );

    expect(Object.keys(store.get(visibleEntriesAtom)).sort()).toEqual(['root', 'worker-1']);
  });

  it('also requests extra items that sit under a collapsed parent', async () => {
    const store = renderBulk([gpu]);

    await waitFor(() =>
      expect(Object.keys(store.get(visibleEntriesAtom)).length).toBeGreaterThan(0)
    );

    expect(Object.keys(store.get(visibleEntriesAtom)).sort()).toEqual([
      'gpu-0',
      'root',
      'worker-1',
    ]);
  });
});
