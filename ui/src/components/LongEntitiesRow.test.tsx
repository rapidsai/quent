// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ButtonHTMLAttributes, HTMLAttributes } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LongEntityDensity } from '@quent/hooks';
import { MAX_TIMELINE_BINS } from '@quent/utils';
import { LongEntitiesRow } from './LongEntitiesRow';

const mocks = vi.hoisted(() => ({
  bulkInitialized: true,
  buildLongEntityEntries: vi.fn((items: unknown[]) => items),
  debouncedZoomRange: { start: 0.2, end: 0.6 },
  getLongEntitiesThreshold: vi.fn(
    (_windowSeconds: number, _numBins: number, density: LongEntityDensity) =>
      ({ 1: 0.15, 2: 0.12, 3: 0.09, 4: 0.06, 5: 0.03 })[density]
  ),
  longEntityDensity: 3 as LongEntityDensity,
  returnedNumBins: 400 as number | undefined,
  returnedTimelineIsStale: false,
  longEntitiesGantt: vi.fn(
    (_props: { entries: unknown[]; height: number; minUsageSeconds: number }) => null
  ),
  useEntityList: vi.fn(),
}));

vi.mock('@quent/client', () => ({
  useEntityList: mocks.useEntityList,
}));

vi.mock('@quent/hooks', () => ({
  useBulkInitialized: () => mocks.bulkInitialized,
  useDebouncedZoomRange: () => mocks.debouncedZoomRange,
  useLongEntityDensity: () => mocks.longEntityDensity,
  useReturnedTimelineIsStale: () => mocks.returnedTimelineIsStale,
  useReturnedTimelineNumBins: () => mocks.returnedNumBins,
  useSelectedNodeIds: () => new Set(['operator-1']),
}));

vi.mock('@quent/components', () => ({
  Button: ({ children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) => (
    <button {...props}>{children}</button>
  ),
  LONG_ENTITIES_TIMELINE_HEIGHT: 110,
  LongEntitiesGantt: (props: { entries: unknown[]; height: number; minUsageSeconds: number }) => {
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
    mocks.bulkInitialized = true;
    mocks.debouncedZoomRange = { start: 0.2, end: 0.6 };
    mocks.longEntityDensity = 3;
    mocks.returnedNumBins = 400;
    mocks.returnedTimelineIsStale = false;
    mocks.useEntityList.mockReturnValue({
      data: undefined,
      isFetching: false,
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
    expect(mocks.getLongEntitiesThreshold.mock.calls[0]?.[1]).toBe(400);
    expect(mocks.getLongEntitiesThreshold.mock.calls[0]?.[2]).toBe(3);
    expect(mocks.useEntityList).toHaveBeenCalledWith(
      expect.objectContaining({
        window: { start: 0.2, end: 0.6 },
        operatorIds: ['operator-1'],
        minUsageSeconds: 0.09,
        maxItems: 100,
      }),
      { enabled: true }
    );
    expect(mocks.longEntitiesGantt.mock.calls[0]?.[0]).toEqual(
      expect.objectContaining({ height: 110, minUsageSeconds: 0.09 })
    );
  });

  it('uses the selected entity density in the query threshold', () => {
    mocks.longEntityDensity = 1;

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

    expect(mocks.useEntityList).toHaveBeenCalledWith(
      expect.objectContaining({ minUsageSeconds: 0.15 }),
      { enabled: true }
    );
  });

  it('falls back to twice the maximum bin count when the timeline request fails', () => {
    mocks.returnedNumBins = undefined;

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
    expect(mocks.getLongEntitiesThreshold.mock.calls[0]?.slice(1)).toEqual([
      MAX_TIMELINE_BINS * 2,
      3,
    ]);
    expect(mocks.useEntityList).toHaveBeenCalledWith(
      expect.objectContaining({ minUsageSeconds: 0.09 }),
      { enabled: true }
    );
    expect(screen.getByTestId('long-entities-gantt')).toBeInTheDocument();
  });

  it('waits for the timeline request before using the fallback', () => {
    mocks.bulkInitialized = false;
    mocks.returnedNumBins = undefined;

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

    expect(mocks.getLongEntitiesThreshold).not.toHaveBeenCalled();
    expect(mocks.useEntityList).toHaveBeenCalledWith(
      expect.objectContaining({ minUsageSeconds: null }),
      { enabled: false }
    );
    expect(screen.getByRole('status', { name: 'Loading entities' })).toBeInTheDocument();
  });

  it('keeps the previous chart visible while a new viewport timeline loads', () => {
    const previousEntity = { id: 'entity-1' };
    mocks.useEntityList.mockReturnValue({
      data: { items: [{ entity: previousEntity, usage_duration_s: 0 }], total: 1 },
      isFetching: false,
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

    mocks.returnedNumBins = undefined;
    mocks.returnedTimelineIsStale = true;
    rerender(<LongEntitiesRow {...props} />);

    expect(mocks.useEntityList).toHaveBeenLastCalledWith(
      expect.objectContaining({ minUsageSeconds: null }),
      { enabled: false }
    );
    expect(screen.queryByRole('status', { name: 'Loading entities' })).not.toBeInTheDocument();
    expect(screen.getByTestId('long-entities-gantt')).toBeInTheDocument();
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
    mocks.useEntityList.mockReturnValue({
      data: undefined,
      isFetching: true,
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

  it('increases the entity limit and keeps it across viewport changes', () => {
    const firstEntity = { id: 'entity-1' };
    const secondEntity = { id: 'entity-2' };
    mocks.useEntityList.mockReturnValue({
      data: { items: [{ entity: firstEntity, usage_duration_s: 0 }], total: 2 },
      isFetching: false,
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
    expect(mocks.useEntityList).toHaveBeenLastCalledWith(
      expect.objectContaining({ maxItems: 200 }),
      { enabled: true }
    );

    mocks.debouncedZoomRange = { start: 0.3, end: 0.7 };
    mocks.useEntityList.mockReturnValue({
      data: {
        items: [
          { entity: firstEntity, usage_duration_s: 0 },
          { entity: secondEntity, usage_duration_s: 0 },
        ],
        total: 2,
      },
      isFetching: false,
      isPlaceholderData: false,
    });
    rerender(<LongEntitiesRow {...props} />);

    expect(mocks.useEntityList).toHaveBeenLastCalledWith(
      expect.objectContaining({
        window: { start: 0.3, end: 0.7 },
        maxItems: 200,
      }),
      { enabled: true }
    );
    expect(mocks.buildLongEntityEntries).toHaveBeenLastCalledWith(
      [firstEntity, secondEntity],
      {},
      'light',
      new Set(['resource-1'])
    );
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('keeps a loading button when more entities will remain', () => {
    const firstEntity = { id: 'entity-1' };
    mocks.useEntityList.mockReturnValue({
      data: { items: [{ entity: firstEntity, usage_duration_s: 0 }], total: 250 },
      isFetching: false,
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

    mocks.useEntityList.mockReturnValue({
      data: { items: [{ entity: firstEntity, usage_duration_s: 0 }], total: 250 },
      isFetching: true,
      isPlaceholderData: true,
    });
    fireEvent.click(screen.getByRole('button', { name: 'Show more (1 of 250)' }));

    expect(screen.getByRole('button', { name: 'Loading...' })).toBeDisabled();

    const secondEntity = { id: 'entity-2' };
    mocks.useEntityList.mockReturnValue({
      data: {
        items: [
          { entity: firstEntity, usage_duration_s: 0 },
          { entity: secondEntity, usage_duration_s: 0 },
        ],
        total: 250,
      },
      isFetching: false,
      isPlaceholderData: false,
    });
    rerender(<LongEntitiesRow {...props} />);

    expect(screen.getByRole('button', { name: 'Show more (2 of 250)' })).toBeEnabled();
  });

  it('keeps the previous entities visible while a changed request loads', () => {
    const previousEntity = { id: 'entity-1' };
    mocks.useEntityList.mockReturnValue({
      data: { items: [{ entity: previousEntity, usage_duration_s: 0 }], total: 2 },
      isFetching: true,
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
      [previousEntity],
      {},
      'light',
      new Set(['resource-1'])
    );
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
