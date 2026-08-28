// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NavBarNavigator } from './NavBarNavigator';

const LONG_QUERY = 'SELECT * FROM lineitem JOIN orders USING (orderkey)';

vi.mock('@tanstack/react-router', async importOriginal => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>();

  return {
    ...actual,
    useMatch: () => ({ params: { engineId: 'engine', queryId: 'query' } }),
    useNavigate: () => vi.fn(),
  };
});

vi.mock('@tanstack/react-query', async importOriginal => {
  const actual = await importOriginal<typeof import('@tanstack/react-query')>();

  return {
    ...actual,
    useQuery: ({ queryKey }: { queryKey: string[] }) => {
      if (queryKey[0] === 'queryBundle') {
        return {
          data: {
            entities: {
              engine: { id: 'engine', instance_name: 'DuckDB' },
              query_group: { id: 'group', instance_name: 'local' },
              query: { id: 'query', instance_name: LONG_QUERY },
            },
          },
        };
      }

      return { data: [] };
    },
    useQueryClient: () => ({ fetchQuery: vi.fn() }),
  };
});

describe('NavBarNavigator', () => {
  it('allows a long query name to truncate', () => {
    render(<NavBarNavigator />);

    const label = screen.getByText(LONG_QUERY);
    const trigger = label.closest('button');
    const navigator = label.closest('nav');

    expect(label).toHaveClass('truncate');
    expect(trigger).toHaveClass('min-w-0');
    expect(navigator).toHaveClass('min-w-0', 'max-w-full');
  });
});
