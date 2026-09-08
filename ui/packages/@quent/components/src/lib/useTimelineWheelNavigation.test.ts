// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { act, renderHook } from '@testing-library/react';
import type { EChartsType } from 'echarts';
import { describe, expect, it, vi } from 'vitest';
import { TIMELINE_SPACING } from '../timeline/types';
import { useTimelineWheelNavigation } from './useTimelineWheelNavigation';

const CHART_WIDTH = 1010;

function createChart(dataZoom: { start: number; end: number }) {
  const dom = document.createElement('div');
  vi.spyOn(dom, 'getBoundingClientRect').mockReturnValue({
    width: CHART_WIDTH,
    height: 100,
    top: 0,
    right: CHART_WIDTH,
    bottom: 100,
    left: 0,
    x: 0,
    y: 0,
    toJSON: () => undefined,
  });

  const dispatchAction = vi.fn();
  const instance = {
    getDom: () => dom,
    getOption: () => ({ dataZoom: [dataZoom] }),
    dispatchAction,
    isDisposed: () => false,
  } as unknown as EChartsType;

  return { instance, dom, dispatchAction };
}

describe('useTimelineWheelNavigation', () => {
  it('pans the visible range on horizontal wheel input', () => {
    const chart = createChart({ start: 25, end: 75 });
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance));

    const event = new WheelEvent('wheel', {
      deltaX: 100,
      bubbles: true,
      cancelable: true,
    });
    act(() => chart.dom.dispatchEvent(event));

    const spanPct = 50;
    const usableWidth = CHART_WIDTH - TIMELINE_SPACING.left - TIMELINE_SPACING.right;
    const expectedStart = 25 + (event.deltaX / usableWidth) * spanPct;
    expect(event.defaultPrevented).toBe(true);
    expect(chart.dispatchAction).toHaveBeenCalledWith({
      type: 'dataZoom',
      dataZoomIndex: 0,
      start: expectedStart,
      end: expectedStart + spanPct,
    });
  });

  it('pans instead of zooming on shifted horizontal wheel input', () => {
    const chart = createChart({ start: 25, end: 75 });
    const wheelTarget = document.createElement('div');
    wheelTarget.appendChild(chart.dom);
    const echartsWheel = vi.fn();
    chart.dom.addEventListener('wheel', echartsWheel);
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance, wheelTarget));

    const event = new WheelEvent('wheel', {
      deltaX: 100,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    });
    act(() => chart.dom.dispatchEvent(event));

    const spanPct = 50;
    const usableWidth = CHART_WIDTH - TIMELINE_SPACING.left - TIMELINE_SPACING.right;
    const expectedStart = 25 + (event.deltaX / usableWidth) * spanPct;
    expect(echartsWheel).not.toHaveBeenCalled();
    expect(chart.dispatchAction).toHaveBeenCalledWith(
      expect.objectContaining({ start: expectedStart, end: expectedStart + spanPct })
    );
  });

  it('allows shifted vertical wheel input to reach ECharts', () => {
    const chart = createChart({ start: 25, end: 75 });
    const wheelTarget = document.createElement('div');
    wheelTarget.appendChild(chart.dom);
    const echartsWheel = vi.fn();
    chart.dom.addEventListener('wheel', echartsWheel);
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance, wheelTarget));

    act(() => {
      chart.dom.dispatchEvent(
        new WheelEvent('wheel', {
          deltaY: 100,
          shiftKey: true,
          bubbles: true,
          cancelable: true,
        })
      );
    });

    expect(echartsWheel).toHaveBeenCalledOnce();
    expect(chart.dispatchAction).not.toHaveBeenCalled();
  });

  it('leaves vertical wheel input to native scrolling', () => {
    const chart = createChart({ start: 25, end: 75 });
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance));

    const event = new WheelEvent('wheel', {
      deltaY: 100,
      bubbles: true,
      cancelable: true,
    });
    act(() => chart.dom.dispatchEvent(event));

    expect(event.defaultPrevented).toBe(false);
    expect(chart.dispatchAction).not.toHaveBeenCalled();
  });

  it('uses the latest minimum span when blocking zoom-in panning', () => {
    const chart = createChart({ start: 20, end: 30 });
    const parent = document.createElement('div');
    parent.appendChild(chart.dom);
    const parentWheel = vi.fn();
    parent.addEventListener('wheel', parentWheel);

    const { result, rerender } = renderHook(
      ({ minZoomSpanPct }) => useTimelineWheelNavigation(minZoomSpanPct),
      { initialProps: { minZoomSpanPct: 5 } }
    );
    act(() => result.current(chart.instance));
    rerender({ minZoomSpanPct: 10 });

    const zoomIn = new WheelEvent('wheel', {
      deltaY: -1,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    });
    act(() => chart.dom.dispatchEvent(zoomIn));

    expect(zoomIn.defaultPrevented).toBe(true);
    expect(parentWheel).not.toHaveBeenCalled();

    const zoomOut = new WheelEvent('wheel', {
      deltaY: 1,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    });
    act(() => chart.dom.dispatchEvent(zoomOut));

    expect(zoomOut.defaultPrevented).toBe(false);
    expect(parentWheel).toHaveBeenCalledOnce();
  });

  it.each([
    { dataZoom: { start: 0, end: 50 }, deltaX: -100, expectedStart: 0 },
    { dataZoom: { start: 50, end: 100 }, deltaX: 100, expectedStart: 50 },
  ])('clamps panning at the range edge for $dataZoom', ({ dataZoom, deltaX, expectedStart }) => {
    const chart = createChart(dataZoom);
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance));

    act(() => {
      chart.dom.dispatchEvent(new WheelEvent('wheel', { deltaX, bubbles: true, cancelable: true }));
    });

    expect(chart.dispatchAction).toHaveBeenCalledWith(
      expect.objectContaining({ start: expectedStart, end: expectedStart + 50 })
    );
  });

  it('removes the previous listener when attach is called again', () => {
    const chart = createChart({ start: 25, end: 75 });
    const addSpy = vi.spyOn(chart.dom, 'addEventListener');
    const removeSpy = vi.spyOn(chart.dom, 'removeEventListener');
    const { result } = renderHook(() => useTimelineWheelNavigation(10));

    act(() => result.current(chart.instance));
    act(() => result.current(chart.instance));

    expect(addSpy.mock.calls.filter(([type]) => type === 'wheel')).toHaveLength(2);
    expect(removeSpy.mock.calls.filter(([type]) => type === 'wheel')).toHaveLength(1);

    act(() => {
      chart.dom.dispatchEvent(
        new WheelEvent('wheel', { deltaX: 100, bubbles: true, cancelable: true })
      );
    });

    expect(chart.dispatchAction).toHaveBeenCalledOnce();
  });

  it('returns a cleanup function for removing the listener explicitly', () => {
    const chart = createChart({ start: 25, end: 75 });
    const { result } = renderHook(() => useTimelineWheelNavigation(10));
    const cleanup = result.current(chart.instance);
    cleanup();

    act(() => {
      chart.dom.dispatchEvent(
        new WheelEvent('wheel', { deltaX: 100, bubbles: true, cancelable: true })
      );
    });

    expect(chart.dispatchAction).not.toHaveBeenCalled();
  });

  it('removes the wheel listener on unmount', () => {
    const chart = createChart({ start: 25, end: 75 });
    const { result, unmount } = renderHook(() => useTimelineWheelNavigation(10));
    act(() => result.current(chart.instance));
    unmount();

    act(() => {
      chart.dom.dispatchEvent(
        new WheelEvent('wheel', { deltaX: 100, bubbles: true, cancelable: true })
      );
    });

    expect(chart.dispatchAction).not.toHaveBeenCalled();
  });
});
