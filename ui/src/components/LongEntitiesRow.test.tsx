// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ButtonHTMLAttributes, HTMLAttributes } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LongEntitiesRow } from './LongEntitiesRow';

const mocks = vi.hoisted(() => ({
  buildLongEntityEntries: vi.fn((items: unknown[]) => items),
  fetchNextPage: vi.fn(),
  getLongEntitiesThreshold: vi.fn((_windowSeconds: number) => 0.06),
  longEntitiesGantt: vi.fn((_props: { entries: unknown[]; height: number }) => null),
  useInfiniteEntityList: vi.fn(),
}));

vi.mock('@quent/client', () => ({
  useInfiniteEntityList: mocks.useInfiniteEntityList,
}));

vi.mock('@quent/hooks', () => ({
  useDebouncedZoomRange: () => ({ start: 0.2, end: 0.6 }),
  useSelectedNodeIds: () => new Set(['operator-1']),
}));

vi.mock('@quent/components', () => ({
  Button: ({ children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) => (
    <button {...props}>{children}</button>
  ),
  LONG_ENTITIES_TIMELINE_HEIGHT: 110,
  LongEntitiesGantt: (props: { entries: unknown[]; height: number }) => {
    mocks.longEntitiesGantt(props);
    return <div data-testid="long-entities-gantt" />;
  },
  Skeleton: (props: HTMLAttributes<HTMLDivElement>) => <div {...props} />,
  buildLongEntityEntries: mocks.buildLongEntityEntries,
  getLongEntitiesThreshold: mocks.getLongEntitiesThreshold,
}));

describe('LongEntitiesRow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.useInfiniteEntityList.mockReturnValue({
      data: undefined,
      fetchNextPage: mocks.fetchNextPage,
      hasNextPage: false,
      isFetching: false,
      isFetchingNextPage: false,
      isPlaceholderData: false,
    });
  });

  it('filters the entity request using the visible timeline window', () => {
    render(
      <LongEntitiesRow
        engineId="engine-1"
        queryId="query-1"
        resourceId="resource-1"
        durationSeconds={1}
        fsmTypes={{}}
        isDark={false}
      />
    );

    expect(mocks.getLongEntitiesThreshold.mock.calls[0]?.[0]).toBeCloseTo(0.4);
    expect(mocks.useInfiniteEntityList).toHaveBeenCalledWith(
      expect.objectContaining({
        window: { start: 0.2, end: 0.6 },
        operatorIds: ['operator-1'],
        minUsageSeconds: 0.06,
        maxItems: 100,
      })
    );
    expect(mocks.longEntitiesGantt.mock.calls[0]?.[0]).toEqual(
      expect.objectContaining({ height: 110 })
    );
  });

  it('can limit FSM states to those used on the associated resource', () => {
    render(
      <LongEntitiesRow
        engineId="engine-1"
        queryId="query-1"
        resourceId="resource-1"
        durationSeconds={1}
        fsmTypes={{}}
        isDark={false}
        fsmStateScope="resource"
      />
    );

    expect(mocks.buildLongEntityEntries).toHaveBeenLastCalledWith(
      [],
      {},
      'light',
      new Set(['resource-1'])
    );
  });

  it('renders a chart-shaped skeleton during the initial load', () => {
    mocks.useInfiniteEntityList.mockReturnValue({
      data: undefined,
      fetchNextPage: mocks.fetchNextPage,
      hasNextPage: false,
      isFetching: true,
      isFetchingNextPage: false,
      isPlaceholderData: false,
    });

    render(
      <LongEntitiesRow
        engineId="engine-1"
        queryId="query-1"
        resourceId="resource-1"
        durationSeconds={1}
        fsmTypes={{}}
        isDark={false}
      />
    );

    const skeleton = screen.getByRole('status', { name: 'Loading entities' });
    expect(skeleton.children).toHaveLength(3);
    expect(screen.queryByText('Loading entities…')).not.toBeInTheDocument();
  });

  it('loads the next page and appends its entities', () => {
    const firstEntity = { entity: { id: 'entity-1' }, usage_duration_s: 0 };
    const secondEntity = { entity: { id: 'entity-2' }, usage_duration_s: 0 };
    mocks.useInfiniteEntityList.mockReturnValue({
      data: { pages: [{ items: [firstEntity], total: 2 }] },
      fetchNextPage: mocks.fetchNextPage,
      hasNextPage: true,
      isFetching: false,
      isFetchingNextPage: false,
      isPlaceholderData: false,
    });

    const props = {
      engineId: 'engine-1',
      queryId: 'query-1',
      resourceId: 'resource-1',
      durationSeconds: 1,
      fsmTypes: {},
      isDark: false,
    };
    const { rerender } = render(<LongEntitiesRow {...props} />);

    const button = screen.getByRole('button', { name: 'Show more (1 of 2)' });
    expect(screen.getByTestId('long-entities-gantt').nextElementSibling).toContainElement(button);
    fireEvent.click(button);
    expect(mocks.fetchNextPage).toHaveBeenCalledOnce();

    mocks.useInfiniteEntityList.mockReturnValue({
      data: {
        pages: [
          { items: [firstEntity], total: 2 },
          { items: [secondEntity], total: 2 },
        ],
      },
      fetchNextPage: mocks.fetchNextPage,
      hasNextPage: false,
      isFetching: false,
      isFetchingNextPage: false,
      isPlaceholderData: false,
    });
    rerender(<LongEntitiesRow {...props} />);

    expect(mocks.buildLongEntityEntries).toHaveBeenLastCalledWith(
      [firstEntity.entity, secondEntity.entity],
      {},
      'light',
      new Set(['resource-1'])
    );
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('keeps the previous entities visible while a changed request loads', () => {
    const previousEntity = { entity: { id: 'entity-1' }, usage_duration_s: 0 };
    mocks.useInfiniteEntityList.mockReturnValue({
      data: { pages: [{ items: [previousEntity], total: 2 }] },
      fetchNextPage: mocks.fetchNextPage,
      hasNextPage: true,
      isFetching: true,
      isFetchingNextPage: false,
      isPlaceholderData: true,
    });

    render(
      <LongEntitiesRow
        engineId="engine-1"
        queryId="query-1"
        resourceId="resource-1"
        durationSeconds={1}
        fsmTypes={{}}
        isDark={false}
      />
    );

    expect(screen.queryByText('Loading entities…')).not.toBeInTheDocument();
    expect(mocks.buildLongEntityEntries).toHaveBeenLastCalledWith(
      [previousEntity.entity],
      {},
      'light',
      new Set(['resource-1'])
    );
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
