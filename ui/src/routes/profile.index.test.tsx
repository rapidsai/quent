// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vite-plus/test';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/mocks/server';
import { renderWithRouter, screen, userEvent, waitFor } from '@/test/test-utils';

const API_BASE = 'http://localhost:8000/api';

const enginesResponse = {
  items: [
    {
      id: 'engine-1',
      instance_name: 'Alpha engine',
      start_time_unix_ns: null,
      duration_s: 12,
      custom_attributes: [
        { key: 'workers', value: { U64: 2 } },
        { key: 'frontend', value: { String: 'ray' } },
        { key: 'num_workloads', value: { U64: 2 } },
        { key: 'num_queries', value: { U64: 8 } },
      ],
      implementation: { name: 'Simulator', version: 'vibe', custom_attributes: [] },
    },
    {
      id: 'engine-2',
      instance_name: 'Beta engine',
      start_time_unix_ns: null,
      duration_s: null,
      custom_attributes: [
        { key: 'workers', value: { U64: 4 } },
        { key: 'frontend', value: { String: 'spmd' } },
        { key: 'num_workloads', value: { U64: 3 } },
        { key: 'num_queries', value: { U64: 12 } },
      ],
      implementation: null,
    },
  ],
  initial_group_by_attribute: 'workers',
};

const queriesResponse = {
  items: [
    {
      id: 'query-1',
      instance_name: 'Q42',
      custom_attributes: [{ key: 'workload', value: { String: 'nightly' } }],
      start_unix_ns: 1_000_000_000,
      planning_s: 0,
      executing_s: 0.1,
      completed_s: 1.5,
    },
  ],
  initial_group_by_attribute: 'workload',
};

describe('EngineSelectionPage', () => {
  beforeEach(() => {
    server.use(
      http.get(`${API_BASE}/engines`, () => HttpResponse.json(enginesResponse)),
      http.get(`${API_BASE}/engines/:engineId/queries`, () => HttpResponse.json(queriesResponse))
    );
  });

  it('renders a searchable, analyzer-grouped engine catalog', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    expect(await screen.findByRole('heading', { name: 'Select an engine' })).toBeInTheDocument();
    await screen.findByText('Alpha engine');
    expect(screen.getByText(/Workers: 2/)).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Num Workloads' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Num Queries' })).toBeInTheDocument();
    expect(screen.getByText('8')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
    expect(screen.getByText('Beta engine')).toBeInTheDocument();

    await user.type(screen.getByRole('textbox', { name: 'Search engine' }), 'ray');

    expect(screen.getByText('Alpha engine')).toBeInTheDocument();
    expect(screen.queryByText('Beta engine')).not.toBeInTheDocument();
  });

  it('defaults to no grouping without an analyzer hint', async () => {
    server.use(
      http.get(`${API_BASE}/engines`, () =>
        HttpResponse.json({ ...enginesResponse, initial_group_by_attribute: null })
      )
    );
    renderWithRouter();

    await screen.findByText('Alpha engine');
    expect(screen.getByRole('combobox', { name: 'Group by' })).toHaveTextContent('None');
    expect(screen.queryByText(/Workers: 2/)).not.toBeInTheDocument();
  });

  it('advances to the query catalog and can return to engines', async () => {
    const user = userEvent.setup();
    const { router } = renderWithRouter();

    await screen.findByText('Alpha engine');
    const engineRow = screen.getByText('Alpha engine').closest('tr');
    if (!engineRow) {
      throw new Error('Engine row was not rendered');
    }
    expect(engineRow.querySelectorAll('td > a')).toHaveLength(
      engineRow.querySelectorAll('td').length
    );
    await user.click(screen.getByRole('link', { name: 'View queries: Alpha engine' }));

    expect(await screen.findByRole('heading', { name: 'Select a query' })).toBeInTheDocument();
    expect(router.state.location.pathname).toBe('/profile/engine/engine-1');
    expect(screen.getByText('Q42')).toBeInTheDocument();
    expect(screen.getByText(/Workload: nightly/)).toBeInTheDocument();
    expect(screen.getByText('Alpha engine')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'View profile: Q42' })).toHaveTextContent('Q42');

    await user.click(screen.getByRole('button', { name: /change engine/i }));
    expect(await screen.findByRole('heading', { name: 'Select an engine' })).toBeInTheDocument();
    expect(router.state.location.pathname).toBe('/');
  });

  it('shows an empty engine catalog', async () => {
    server.use(
      http.get(`${API_BASE}/engines`, () =>
        HttpResponse.json({ items: [], initial_group_by_attribute: null })
      )
    );
    renderWithRouter();

    expect(await screen.findByText('No engines are available.')).toBeInTheDocument();
  });

  it('shows errors and retries the request', async () => {
    let requests = 0;
    server.use(
      http.get(`${API_BASE}/engines`, () => {
        requests += 1;
        if (requests === 1) {
          return new HttpResponse(null, { status: 500, statusText: 'Internal Server Error' });
        }
        return HttpResponse.json(enginesResponse);
      })
    );
    const user = userEvent.setup();
    renderWithRouter();

    expect(await screen.findByRole('alert')).toHaveTextContent('Unable to load this catalog');
    await user.click(screen.getByRole('button', { name: /retry/i }));

    await waitFor(() => expect(screen.getByText('Alpha engine')).toBeInTheDocument());
  });
});
