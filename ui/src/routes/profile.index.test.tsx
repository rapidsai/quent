// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, beforeEach } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/mocks/server';
import { screen, renderWithRouter, waitFor, within, fireEvent } from '@/test/test-utils';

const API_BASE = 'http://localhost:8000/api';

function makeQuery(id: string, name: string, groupId: string) {
  return {
    id,
    query_group_id: groupId,
    instance_name: name,
    start_unix_ns: null,
    planning_s: 0.5,
    executing_s: 1.5,
    completed_s: 3,
  };
}

function installProfileHandlers() {
  server.use(
    http.get(`${API_BASE}/engines`, () => {
      return HttpResponse.json([
        { id: 'engine-1', instance_name: 'Engine One' },
        { id: 'engine-2', instance_name: 'Engine Two' },
      ]);
    }),
    http.get(`${API_BASE}/engines/:engineId/query-groups`, ({ params }) => {
      const { engineId } = params;
      return HttpResponse.json([
        { id: `${engineId}-group-1`, instance_name: `Group ${engineId}`, engine_id: engineId },
      ]);
    }),
    http.get(`${API_BASE}/engines/:engineId/query_group/:groupId/queries`, ({ params }) => {
      const { engineId, groupId } = params;
      if (engineId === 'engine-1') {
        return HttpResponse.json([
          makeQuery('q-alpha', 'Alpha Query', String(groupId)),
          makeQuery('q-beta', 'Beta Query', String(groupId)),
        ]);
      }
      return HttpResponse.json([makeQuery('q-gamma', 'Gamma Query', String(groupId))]);
    })
  );
}

describe('ProfileSearchPage', () => {
  beforeEach(() => {
    installProfileHandlers();
  });

  it('renders the page title and description', async () => {
    renderWithRouter({ initialPath: '/profile' });

    expect(await screen.findByRole('heading', { name: /search profiles/i })).toBeInTheDocument();
    expect(screen.getByText(/search and filter query profiles/i)).toBeInTheDocument();
  });

  it('renders the search box and filter controls', async () => {
    renderWithRouter({ initialPath: '/profile' });

    await screen.findByRole('heading', { name: /search profiles/i });
    expect(screen.getByLabelText(/search profiles/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/filter by engine/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/filter by query group/i)).toBeInTheDocument();
  });

  it('aggregates and lists all profiles across engines', async () => {
    renderWithRouter({ initialPath: '/profile' });

    await waitFor(() => {
      expect(screen.getByText('Alpha Query')).toBeInTheDocument();
    });
    expect(screen.getByText('Beta Query')).toBeInTheDocument();
    expect(screen.getByText('Gamma Query')).toBeInTheDocument();
    expect(screen.getByText(/3 profiles/i)).toBeInTheDocument();
  });

  it('filters rows by the free-text search box', async () => {
    renderWithRouter({ initialPath: '/profile' });

    await waitFor(() => {
      expect(screen.getByText('Alpha Query')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/search profiles/i), {
      target: { value: 'gamma' },
    });

    await waitFor(() => {
      expect(screen.queryByText('Alpha Query')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Gamma Query')).toBeInTheDocument();
  });

  it('shows an empty state when no profiles are available', async () => {
    server.use(http.get(`${API_BASE}/engines`, () => HttpResponse.json([])));

    renderWithRouter({ initialPath: '/profile' });

    await waitFor(() => {
      expect(screen.getByText(/no profiles available/i)).toBeInTheDocument();
    });
  });

  it('navigates to a profile when a row is clicked', async () => {
    const { router } = renderWithRouter({ initialPath: '/profile' });

    await waitFor(() => {
      expect(screen.getByText('Alpha Query')).toBeInTheDocument();
    });

    const table = screen.getByRole('table');
    fireEvent.click(within(table).getByText('Alpha Query'));

    await waitFor(() => {
      expect(router.state.location.pathname).toContain('/profile/engine/engine-1/query/q-alpha');
    });
  });
});
