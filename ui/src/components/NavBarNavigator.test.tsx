// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { NavBarNavigator } from './NavBarNavigator';
import { act, renderWithQuery, screen, userEvent, waitFor } from '@/test/test-utils';

const { fetchQueryMock, navigateMock, routeParams, useQueryMock } = vi.hoisted(() => ({
  fetchQueryMock: vi.fn(),
  navigateMock: vi.fn(),
  routeParams: {
    current: { engineId: 'engine-current', queryId: 'query-current' },
  },
  useQueryMock: vi.fn(),
}));

vi.mock('@tanstack/react-router', async importOriginal => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>();
  return {
    ...actual,
    Link: ({ children }: { children: React.ReactNode }) => children,
    useMatch: () => ({ params: routeParams.current }),
    useNavigate: () => navigateMock,
  };
});

vi.mock('@tanstack/react-query', async importOriginal => {
  const actual = await importOriginal<typeof import('@tanstack/react-query')>();
  return {
    ...actual,
    useQuery: useQueryMock,
    useQueryClient: () => ({ fetchQuery: fetchQueryMock }),
  };
});

const queryBundle = {
  entities: {
    engine: {
      id: 'engine-current',
      instance_name: 'Current engine',
      custom_attributes: [],
    },
    query: {
      id: 'query-current',
      instance_name: 'Current query',
      custom_attributes: [],
    },
  },
};

const engines = {
  items: [
    { id: 'engine-current', instance_name: 'Current engine', custom_attributes: [] },
    { id: 'engine-a', instance_name: 'Engine A', custom_attributes: [] },
    { id: 'engine-b', instance_name: 'Engine B', custom_attributes: [] },
  ],
};

const queries = {
  items: [{ id: 'query-current', instance_name: 'Current query', custom_attributes: [] }],
};

describe('NavBarNavigator', () => {
  beforeEach(() => {
    navigateMock.mockReset();
    fetchQueryMock.mockReset();
    routeParams.current = { engineId: 'engine-current', queryId: 'query-current' };
    useQueryMock.mockImplementation(options => {
      switch (options.queryKey[0]) {
        case 'queryBundle':
          return { data: queryBundle, isLoading: false, isError: false };
        case 'list_engines':
          return { data: engines, isLoading: false, isError: false };
        case 'list_queries':
          return { data: queries, isLoading: false, isError: false };
        default:
          throw new Error(`Unexpected query key: ${options.queryKey[0]}`);
      }
    });
  });

  it('ignores stale switches and clears errors when the route changes', async () => {
    let resolveEngineA!: (value: typeof queries) => void;
    let resolveEngineB!: (value: typeof queries) => void;
    const engineA = new Promise<typeof queries>(resolve => {
      resolveEngineA = resolve;
    });
    const engineB = new Promise<typeof queries>(resolve => {
      resolveEngineB = resolve;
    });
    fetchQueryMock.mockImplementation(options =>
      options.queryKey[1] === 'engine-a' ? engineA : engineB
    );

    const user = userEvent.setup();
    const { rerender } = renderWithQuery(<NavBarNavigator />);

    await user.click(screen.getByRole('button', { name: 'Change Current engine' }));
    await user.click(screen.getByRole('menuitem', { name: 'Engine A' }));
    await user.click(screen.getByRole('button', { name: 'Change Current engine' }));
    await user.click(screen.getByRole('menuitem', { name: 'Engine B' }));

    await act(async () => {
      resolveEngineB({ items: [] });
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('That engine has no queries.');

    routeParams.current = { engineId: 'engine-current', queryId: 'query-next' };
    rerender(<NavBarNavigator />);
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());

    await act(async () => {
      resolveEngineA({
        items: [{ id: 'query-a', instance_name: 'Query A', custom_attributes: [] }],
      });
    });
    expect(navigateMock).not.toHaveBeenCalled();
  });
});
