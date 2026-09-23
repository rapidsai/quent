// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { fireEvent, render } from '@testing-library/react';
import type { EChartsInstance } from 'echarts-for-react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlayheadLinePixel } from '../lib/usePlayheadLinePixel';
import { PlayheadLine } from './PlayheadLine';

const mocks = vi.hoisted(() => ({
  isPlaying: false,
  pixelX: 24 as number | null,
  setIsPlaying: vi.fn(),
  setPlayheadLineTimeMs: vi.fn(),
  setPlayheadTimeS: vi.fn(),
  zoomRange: { start: 0, end: 100 },
}));

vi.mock('@quent/hooks', () => ({
  useDataFlowIsPlaying: () => mocks.isPlaying,
  useSetDataFlowIsPlaying: () => mocks.setIsPlaying,
  useSetPlayheadLineTimeMs: () => mocks.setPlayheadLineTimeMs,
  useSetPlayheadTimeS: () => mocks.setPlayheadTimeS,
  useZoomRange: () => mocks.zoomRange,
}));

vi.mock('../lib/usePlayheadLinePixel', () => ({
  usePlayheadLinePixel: vi.fn(() => mocks.pixelX),
}));

describe('PlayheadLine', () => {
  afterEach(() => {
    mocks.isPlaying = false;
    mocks.pixelX = 24;
    mocks.setIsPlaying.mockReset();
    mocks.setPlayheadLineTimeMs.mockReset();
    mocks.setPlayheadTimeS.mockReset();
    mocks.zoomRange = { start: 0, end: 100 };
    vi.mocked(usePlayheadLinePixel).mockClear();
    vi.unstubAllGlobals();
  });

  it('renders nothing when the playhead has no pixel position', () => {
    mocks.pixelX = null;
    const { container } = render(<PlayheadLine instance={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('only renders the top indicator when requested', () => {
    const { container, rerender } = render(<PlayheadLine instance={null} />);
    expect(container.querySelector('[data-playhead-indicator]')).not.toBeInTheDocument();

    rerender(<PlayheadLine instance={null} showIndicator />);
    expect(container.querySelector('[data-playhead-indicator]')).toBeInTheDocument();
  });

  it('positions the overlay at the computed pixel, including zero', () => {
    const { container, rerender } = render(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).toHaveStyle({ left: '24px' });

    mocks.pixelX = 0;
    rerender(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).toHaveStyle({ left: '0px' });
  });

  it('only transitions position during data-flow playback', () => {
    const { container, rerender } = render(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).not.toHaveClass('transition-[left]');

    mocks.isPlaying = true;
    rerender(<PlayheadLine instance={null} />);

    expect(container.firstElementChild).toHaveClass(
      'transition-[left]',
      'duration-100',
      'ease-linear',
      'motion-reduce:transition-none'
    );

    mocks.isPlaying = false;
    rerender(<PlayheadLine instance={null} />);
    expect(container.firstElementChild).not.toHaveClass('transition-[left]');
  });

  it('forwards the chart instance and x-axis index', () => {
    const instance = {} as EChartsInstance;
    const { rerender } = render(<PlayheadLine instance={instance} />);
    expect(usePlayheadLinePixel).toHaveBeenCalledWith(instance, 0);

    rerender(<PlayheadLine instance={instance} xAxisIndex={1} />);
    expect(usePlayheadLinePixel).toHaveBeenCalledWith(instance, 1);
  });

  it('captures a line drag and updates both playhead times from chart coordinates', () => {
    const chartDom = document.createElement('div');
    vi.spyOn(chartDom, 'getBoundingClientRect').mockReturnValue({
      left: 10,
      width: 100,
    } as DOMRect);
    const convertFromPixel = vi.fn(
      (_finder: { xAxisIndex: number }, offsetX: number) => offsetX * 100
    );
    const instance = {
      convertFromPixel,
      getDom: () => chartDom,
      isDisposed: () => false,
    } as unknown as EChartsInstance;
    const { container } = render(<PlayheadLine instance={instance} xAxisIndex={2} draggable />);
    const dragArea = container.firstElementChild as HTMLDivElement;
    dragArea.setPointerCapture = vi.fn();
    dragArea.hasPointerCapture = vi.fn(() => true);
    dragArea.releasePointerCapture = vi.fn();

    fireEvent.pointerDown(dragArea, { clientX: 50, pointerId: 7 });

    expect(dragArea.setPointerCapture).toHaveBeenCalledWith(7);
    expect(mocks.setIsPlaying).toHaveBeenCalledWith(false);
    expect(convertFromPixel).toHaveBeenCalledWith({ xAxisIndex: 2 }, 40);
    expect(mocks.setPlayheadLineTimeMs).toHaveBeenCalledWith(4000);
    expect(mocks.setPlayheadTimeS).toHaveBeenCalledWith(4);
  });

  it('continues updating while the captured pointer crosses timeline rows', () => {
    const animationFrames: FrameRequestCallback[] = [];
    vi.stubGlobal(
      'requestAnimationFrame',
      vi.fn((callback: FrameRequestCallback) => {
        animationFrames.push(callback);
        return animationFrames.length;
      })
    );
    vi.stubGlobal('cancelAnimationFrame', vi.fn());

    const chartDom = document.createElement('div');
    vi.spyOn(chartDom, 'getBoundingClientRect').mockReturnValue({
      left: 0,
      width: 100,
    } as DOMRect);
    const instance = {
      convertFromPixel: vi.fn((_finder: unknown, offsetX: number) => offsetX * 10),
      getDom: () => chartDom,
      isDisposed: () => false,
    } as unknown as EChartsInstance;
    const { container } = render(<PlayheadLine instance={instance} draggable />);
    const dragArea = container.firstElementChild as HTMLDivElement;
    dragArea.setPointerCapture = vi.fn();
    dragArea.hasPointerCapture = vi.fn(() => true);
    dragArea.releasePointerCapture = vi.fn();

    fireEvent.pointerDown(dragArea, { clientX: 20, clientY: 10, pointerId: 3 });
    mocks.setPlayheadLineTimeMs.mockClear();
    mocks.setPlayheadTimeS.mockClear();

    fireEvent.pointerMove(dragArea, { clientX: 70, clientY: 200, pointerId: 3 });
    animationFrames[0]?.(0);

    expect(mocks.setPlayheadLineTimeMs).toHaveBeenLastCalledWith(700);
    expect(mocks.setPlayheadTimeS).toHaveBeenLastCalledWith(0.7);

    fireEvent.pointerUp(dragArea, { clientX: 80, clientY: 200, pointerId: 3 });

    expect(mocks.setPlayheadLineTimeMs).toHaveBeenLastCalledWith(800);
    expect(dragArea.releasePointerCapture).toHaveBeenCalledWith(3);
  });

  it('constrains dragging to the current timeline viewport', () => {
    mocks.zoomRange = { start: 2, end: 5 };
    const chartDom = document.createElement('div');
    vi.spyOn(chartDom, 'getBoundingClientRect').mockReturnValue({
      left: 0,
      width: 100,
    } as DOMRect);
    const instance = {
      convertFromPixel: vi.fn((_finder: unknown, offsetX: number) => offsetX * 100),
      getDom: () => chartDom,
      isDisposed: () => false,
    } as unknown as EChartsInstance;
    const { container } = render(<PlayheadLine instance={instance} draggable />);
    const dragArea = container.firstElementChild as HTMLDivElement;
    dragArea.setPointerCapture = vi.fn();
    dragArea.hasPointerCapture = vi.fn(() => true);
    dragArea.releasePointerCapture = vi.fn();

    fireEvent.pointerDown(dragArea, { clientX: 10, pointerId: 5 });
    expect(mocks.setPlayheadLineTimeMs).toHaveBeenLastCalledWith(2000);
    expect(mocks.setPlayheadTimeS).toHaveBeenLastCalledWith(2);

    fireEvent.pointerUp(dragArea, { clientX: 90, pointerId: 5 });
    expect(mocks.setPlayheadLineTimeMs).toHaveBeenLastCalledWith(5000);
    expect(mocks.setPlayheadTimeS).toHaveBeenLastCalledWith(5);
  });

  it('remains display-only when dragging is disabled', () => {
    const convertFromPixel = vi.fn();
    const instance = {
      convertFromPixel,
      getDom: () => document.createElement('div'),
      isDisposed: () => false,
    } as unknown as EChartsInstance;
    const { container } = render(<PlayheadLine instance={instance} draggable={false} />);

    fireEvent.pointerDown(container.firstElementChild as Element, { clientX: 20, pointerId: 1 });

    expect(convertFromPixel).not.toHaveBeenCalled();
    expect(mocks.setIsPlaying).not.toHaveBeenCalled();
    expect(mocks.setPlayheadLineTimeMs).not.toHaveBeenCalled();
  });
});
