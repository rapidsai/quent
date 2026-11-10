// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { http, HttpResponse } from 'msw';
import { encodeDeepLinkState } from '@/features/deep-link/deepLink.codec';
import { server } from '@/test/mocks/server';
import { renderWithRouter, screen, userEvent, waitFor } from '@/test/test-utils';
import { writeSavedViews } from '@/features/saved-views/savedViews.storage';

vi.mock('@/components/QueryColorRegistry', () => ({
  QueryColorRegistry: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('@/components/QueryPlan', () => ({
  QueryPlan: () => null,
}));

vi.mock('@/components/operator-table/OperatorTable', () => ({
  OperatorTable: () => <div>Operators content</div>,
}));

vi.mock('@/components/timeline-tree', async () => {
  const { useAtomValue } = await import('jotai');
  const { useLongEntityDensity, useSetZoomRange, useZoomRange } = await import('@quent/hooks');
  const { pinnedCollapsedAtom } = await import('@/atoms/resourceTree');
  return {
    QueryResourceTree: () => {
      const zoom = useZoomRange();
      const setZoom = useSetZoomRange();
      const pinnedCollapsed = useAtomValue(pinnedCollapsedAtom);
      const density = useLongEntityDensity();
      return (
        <div>
          <output data-testid="restored-view">
            {JSON.stringify({ zoom, pinnedCollapsed, density })}
          </output>
          <button type="button" onClick={() => setZoom({ start: 1, end: 2 })}>
            Change view
          </button>
        </div>
      );
    },
  };
});

const scope = { engineId: 'engine-a', queryId: 'query-a' };

const queryBundle = {
  query_id: scope.queryId,
  entities: {
    engine: { id: scope.engineId, instance_name: 'Engine A' },
    query: { id: scope.queryId, instance_name: 'Query A' },
    workers: {},
    plans: {},
    operators: {},
    ports: {},
    resource_types: {},
    resource_group_types: {},
    resources: {},
    resource_groups: {},
    fsm_types: {},
  },
  resource_tree: {
    ResourceGroup: {
      id: { Engine: scope.engineId },
      children: [],
    },
  },
  plan_tree: { id: 'plan-a', worker: null, children: [] },
  unique_operator_names: [],
  quantity_specs: {},
  start_time_unix_ns: 10_000_000_000_000_000,
  duration_s: 100,
};

beforeEach(() => {
  localStorage.clear();
  server.use(
    http.get('http://localhost:8000/api/engines/:engineId/query/:queryId', ({ params }) =>
      HttpResponse.json({
        ...queryBundle,
        query_id: params.queryId,
        entities: {
          ...queryBundle.entities,
          query: { id: params.queryId, instance_name: `Query ${params.queryId}` },
        },
      })
    )
  );
});

describe('saved view route restore', () => {
  it('navigates to the saved tab and restores the same view repeatedly', async () => {
    const encoded = encodeDeepLinkState({
      route: { ...scope, tab: 'timeline' },
      timeline: { zoomRange: { start: 20, end: 60 } },
    });
    if (!encoded.ok) {
      throw new Error(encoded.message);
    }
    expect(
      writeSavedViews(localStorage, scope, {
        views: [
          {
            id: '99a8834d-768a-4eb4-9b72-1e7987f8cbda',
            name: 'Timeline investigation',
            createdAt: 1,
            updatedAt: 1,
            tab: 'timeline',
            payload: encoded.value,
            extras: { pinnedCollapsed: true, longEntityDensity: 5 },
          },
        ],
        invalidEntries: [],
      })
    ).toEqual({ ok: true });

    const { router } = renderWithRouter({
      initialPath: `/profile/engine/${scope.engineId}/query/${scope.queryId}/operators`,
    });
    expect(await screen.findByText('Operators content')).toBeVisible();

    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(
      screen.getByRole('button', { name: 'Load Timeline investigation (Timeline)' })
    );

    await waitFor(() =>
      expect(router.state.location.pathname).toBe(
        `/profile/engine/${scope.engineId}/query/${scope.queryId}/timeline`
      )
    );
    expect(await screen.findByTestId('restored-view')).toHaveTextContent(
      JSON.stringify({
        zoom: { start: 20, end: 60 },
        pinnedCollapsed: true,
        density: 5,
      })
    );

    await userEvent.click(screen.getByRole('button', { name: 'Change view' }));
    expect(screen.getByTestId('restored-view')).toHaveTextContent('"start":1');
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(
      screen.getByRole('button', { name: 'Load Timeline investigation (Timeline)' })
    );
    await waitFor(() =>
      expect(screen.getByTestId('restored-view')).toHaveTextContent('"start":20')
    );

    await router.navigate({
      to: '/profile/engine/$engineId/query/$queryId/operators',
      params: { engineId: scope.engineId, queryId: 'query-b' },
      search: {},
    });
    expect(await screen.findByText('Operators content')).toBeVisible();
    expect(router.state.location.pathname).toBe(
      `/profile/engine/${scope.engineId}/query/query-b/operators`
    );
    expect(screen.queryByText('Could not restore saved view')).not.toBeInTheDocument();
  });
});
