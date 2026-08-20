// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { PropsWithChildren } from 'react';
import { renderHook } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import type { OperatorFilter, SingleTimelineResponse, TimelineRequest } from '@quent/utils';
import {
  debouncedZoomRangeAtom,
  timelineCacheKey,
  timelineDataMapAtom,
  visibleEntriesAtom,
} from '../atoms/timeline';
import { useReturnedTimelineIsStale, useReturnedTimelineNumBins } from './useTimelineAtoms';

describe('useReturnedTimelineNumBins', () => {
  it('reads the returned bin count for the visible resource request', () => {
    const store = createStore();
    const request: TimelineRequest<OperatorFilter> = {
      Resource: {
        resource_id: 'resource-1',
        long_entities_threshold_s: null,
        entity_filter: { entity_type_name: 'fsm-1' },
        application: { operator_ids: [] },
        config: { num_bins: 200, start: 0, end: 1 },
      },
    };
    const response: SingleTimelineResponse = {
      config: {
        span: { start: -0.001, end: 1.001 },
        bin_duration: 0.0025,
        num_bins: 400n,
      },
      data: {} as SingleTimelineResponse['data'],
    };
    const cacheKey = timelineCacheKey({
      resourceId: 'resource-1',
      resourceTypeName: '',
      fsmTypeName: 'fsm-1',
    });
    store.set(visibleEntriesAtom, { 'resource-1': request });
    store.set(timelineDataMapAtom, { [cacheKey]: response });
    store.set(debouncedZoomRangeAtom, { start: 0, end: 1 });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(() => useReturnedTimelineNumBins('resource-1'), { wrapper });

    expect(result.current).toBe(400);
  });

  it('returns undefined when no response is cached', () => {
    const store = createStore();
    const request: TimelineRequest<OperatorFilter> = {
      Resource: {
        resource_id: 'resource-1',
        long_entities_threshold_s: null,
        entity_filter: { entity_type_name: 'fsm-1' },
        application: { operator_ids: [] },
        config: { num_bins: 200, start: 0, end: 1 },
      },
    };
    store.set(visibleEntriesAtom, { 'resource-1': request });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(() => useReturnedTimelineNumBins('resource-1'), { wrapper });

    expect(result.current).toBeUndefined();
  });

  it('returns undefined while the cached response belongs to the previous viewport', () => {
    const store = createStore();
    const request: TimelineRequest<OperatorFilter> = {
      Resource: {
        resource_id: 'resource-1',
        long_entities_threshold_s: null,
        entity_filter: { entity_type_name: 'fsm-1' },
        application: { operator_ids: [] },
        config: { num_bins: 200, start: 0.25, end: 1 },
      },
    };
    const cacheKey = timelineCacheKey({
      resourceId: 'resource-1',
      resourceTypeName: '',
      fsmTypeName: 'fsm-1',
    });
    const response: SingleTimelineResponse = {
      config: {
        span: { start: 0, end: 1 },
        bin_duration: 0.0025,
        num_bins: 400n,
      },
      data: {} as SingleTimelineResponse['data'],
    };
    store.set(visibleEntriesAtom, { 'resource-1': request });
    store.set(timelineDataMapAtom, { [cacheKey]: response });
    store.set(debouncedZoomRangeAtom, { start: 0.25, end: 1 });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(
      () => ({
        numBins: useReturnedTimelineNumBins('resource-1'),
        isStale: useReturnedTimelineIsStale('resource-1'),
      }),
      { wrapper }
    );

    expect(result.current).toEqual({ numBins: undefined, isStale: true });
  });
});
