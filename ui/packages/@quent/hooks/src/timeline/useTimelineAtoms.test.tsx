// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { PropsWithChildren } from 'react';
import { act, renderHook } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import type { OperatorFilter, SingleTimelineResponse, TimelineRequest } from '@quent/utils';
import {
  debouncedZoomRangeAtom,
  timelineCacheKey,
  timelineDataMapAtom,
  timelinePointerAtom,
  visibleEntriesAtom,
  zoomRangeAtom,
} from '../atoms/timeline';
import {
  useGetZoomRange,
  useReturnedTimelineIsStale,
  useReturnedTimelineNumBins,
  useTimelinePointerPublisher,
  useTimelinePointerRatio,
  useZeroUtilizationResourceIds,
} from './useTimelineAtoms';

describe('useGetZoomRange', () => {
  it('reads the latest zoom without subscribing the chart to zoom updates', () => {
    const store = createStore();
    store.set(zoomRangeAtom, { start: 0, end: 100 });
    let renderCount = 0;
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );
    const { result } = renderHook(
      () => {
        renderCount += 1;
        return useGetZoomRange();
      },
      { wrapper }
    );

    act(() => store.set(zoomRangeAtom, { start: 25, end: 75 }));

    expect(renderCount).toBe(1);
    expect(result.current()).toEqual({ start: 25, end: 75 });
  });
});

describe('timeline pointer', () => {
  it('publishes a ratio and only clears the current owner', () => {
    const store = createStore();
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.push(callback);
      return frames.length;
    });

    try {
      const { result } = renderHook(
        () => ({
          first: useTimelinePointerPublisher(),
          second: useTimelinePointerPublisher(),
          ratio: useTimelinePointerRatio(),
        }),
        { wrapper }
      );

      act(() => result.current.first.publish(0.25));
      expect(result.current.ratio).toBe(0.25);
      act(() => result.current.first.clear());
      act(() => result.current.first.publish(0.5));
      act(() => frames.shift()?.(0));
      expect(result.current.ratio).toBe(0.5);
      act(() => result.current.second.publish(0.75));
      act(() => result.current.first.clear());
      expect(result.current.ratio).toBe(0.75);
      act(() => result.current.second.clear());
      act(() => frames.shift()?.(0));
      expect(result.current.ratio).toBeNull();
      expect(store.get(timelinePointerAtom)).toBeNull();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

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

describe('useZeroUtilizationResourceIds', () => {
  function makeRequest(resourceId: string): TimelineRequest<OperatorFilter> {
    return {
      Resource: {
        resource_id: resourceId,
        long_entities_threshold_s: null,
        entity_filter: { entity_type_name: null },
        application: { operator_ids: [] },
        config: { num_bins: 10, start: 0, end: 1 },
      },
    };
  }

  function makeResponse(values: number[]): SingleTimelineResponse {
    return {
      config: { span: { start: 0, end: 1 }, bin_duration: 0.1, num_bins: BigInt(values.length) },
      data: { Binned: { capacities_values: { count: values }, long_fsms: [] } } as never,
    };
  }

  it('includes a resource whose current-window utilization is all zero', () => {
    const store = createStore();
    store.set(visibleEntriesAtom, {
      'zero-resource': makeRequest('zero-resource'),
      'busy-resource': makeRequest('busy-resource'),
    });
    store.set(timelineDataMapAtom, {
      [timelineCacheKey({ resourceId: 'zero-resource', resourceTypeName: '' })]: makeResponse([
        0, 0, 0,
      ]),
      [timelineCacheKey({ resourceId: 'busy-resource', resourceTypeName: '' })]: makeResponse([
        0, 3, 0,
      ]),
    });
    store.set(debouncedZoomRangeAtom, { start: 0, end: 1 });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(() => useZeroUtilizationResourceIds(), { wrapper });

    expect(result.current.has('zero-resource')).toBe(true);
    expect(result.current.has('busy-resource')).toBe(false);
  });

  it('excludes a resource whose cached response belongs to a stale viewport', () => {
    const store = createStore();
    store.set(visibleEntriesAtom, { 'zero-resource': makeRequest('zero-resource') });
    store.set(timelineDataMapAtom, {
      [timelineCacheKey({ resourceId: 'zero-resource', resourceTypeName: '' })]: makeResponse([
        0, 0, 0,
      ]),
    });
    store.set(debouncedZoomRangeAtom, { start: 0.5, end: 1 });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(() => useZeroUtilizationResourceIds(), { wrapper });

    expect(result.current.has('zero-resource')).toBe(false);
  });

  it('returns an empty set when no timeline data has been fetched yet', () => {
    const store = createStore();
    store.set(visibleEntriesAtom, { 'zero-resource': makeRequest('zero-resource') });
    const wrapper = ({ children }: PropsWithChildren) => (
      <Provider store={store}>{children}</Provider>
    );

    const { result } = renderHook(() => useZeroUtilizationResourceIds(), { wrapper });

    expect(result.current.size).toBe(0);
  });
});
