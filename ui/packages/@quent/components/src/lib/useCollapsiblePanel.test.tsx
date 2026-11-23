// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook } from '@testing-library/react';
import type { PanelImperativeHandle } from 'react-resizable-panels';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  useCollapsiblePanel,
  type PanelHeights,
  type UseCollapsiblePanelOptions,
} from './useCollapsiblePanel';

const OPEN_HEIGHTS: PanelHeights = { content: 300, fit: 180 };

let frames = new Map<number, FrameRequestCallback>();
let nextFrameId = 1;

function runFrame() {
  const pending = [...frames.entries()];
  frames = new Map();
  pending.forEach(([, callback]) => callback(0));
}

function fakePanel() {
  return {
    resize: vi.fn(),
    collapse: vi.fn(),
    expand: vi.fn(),
    isCollapsed: vi.fn(() => false),
    getSize: vi.fn(() => ({ inPixels: 180, asPercentage: 30 })),
  } satisfies PanelImperativeHandle;
}

function setup(overrides: Partial<UseCollapsiblePanelOptions> = {}) {
  const panel = fakePanel();
  const measure = vi.fn<() => PanelHeights | undefined>(() => OPEN_HEIGHTS);
  const initialProps: UseCollapsiblePanelOptions = {
    enabled: true,
    collapsed: false,
    fitKey: 1,
    measure,
    ...overrides,
  };
  const hook = renderHook(
    (props: UseCollapsiblePanelOptions) => {
      const result = useCollapsiblePanel(props);
      result.panelRef.current = props.enabled ? panel : null;
      return result;
    },
    { initialProps }
  );
  return { panel, measure, initialProps, ...hook };
}

beforeEach(() => {
  frames = new Map();
  nextFrameId = 1;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    const id = nextFrameId++;
    frames.set(id, callback);
    return id;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('useCollapsiblePanel fitting', () => {
  it('waits for the panel to be laid out, then resizes it to fit', () => {
    const { panel } = setup();

    expect(panel.resize).not.toHaveBeenCalled();
    runFrame();

    expect(panel.resize).toHaveBeenLastCalledWith(180);
  });

  it('uses the content height instead of the fit height when it starts collapsed', () => {
    const measure = vi.fn(() => ({ content: 40, fit: 100 }));
    const { panel } = setup({ collapsed: true, measure });
    runFrame();

    expect(panel.resize).toHaveBeenCalledWith(40);
    expect(panel.resize).not.toHaveBeenCalledWith(100);
  });

  it('fits again when the fit key changes', () => {
    const { panel, measure, initialProps, rerender } = setup();
    runFrame();

    measure.mockReturnValue({ content: 400, fit: 260 });
    rerender({ ...initialProps, fitKey: 2 });
    runFrame();

    expect(panel.resize).toHaveBeenLastCalledWith(260);
  });

  it.each(['onPointerDown', 'onKeyDown'] as const)(
    'stops fitting once the user resizes (%s)',
    handler => {
      const { panel, measure, initialProps, result, rerender } = setup();
      runFrame();

      result.current.handleProps[handler]();
      measure.mockReturnValue({ content: 400, fit: 260 });
      rerender({ ...initialProps, fitKey: 2 });
      runFrame();

      expect(panel.resize).not.toHaveBeenCalledWith(260);
    }
  );

  it('fits again after the panel goes away and comes back', () => {
    const { panel, measure, initialProps, result, rerender } = setup();
    runFrame();
    result.current.handleProps.onPointerDown();

    rerender({ ...initialProps, enabled: false });
    measure.mockReturnValue({ content: 400, fit: 260 });
    rerender({ ...initialProps, enabled: true });
    runFrame();

    expect(panel.resize).toHaveBeenLastCalledWith(260);
  });

  it('does not measure while disabled', () => {
    const { measure } = setup({ enabled: false });
    runFrame();

    expect(measure).not.toHaveBeenCalled();
  });

  it.each([
    ['nothing to measure', undefined],
    ['content with no height yet', { content: 0, fit: 0 }],
  ])('leaves the panel alone when there is %s', (_label, heights) => {
    const measure = vi.fn(() => heights);
    const { panel } = setup({ measure });
    runFrame();

    expect(panel.resize).not.toHaveBeenCalled();
  });
});

describe('useCollapsiblePanel collapsing', () => {
  it('shrinks to the content height, then restores the height it had when opened', () => {
    const { panel, measure, initialProps, rerender } = setup();
    runFrame();

    panel.getSize.mockReturnValue({ inPixels: 220, asPercentage: 35 });
    measure.mockReturnValue({ content: 40, fit: 40 });
    rerender({ ...initialProps, collapsed: true });
    runFrame();
    expect(panel.resize).toHaveBeenLastCalledWith(40);

    rerender({ ...initialProps, collapsed: false });
    runFrame();
    expect(panel.resize).toHaveBeenLastCalledWith(220);
  });

  it('keeps the panel as it is when collapsing and nothing can be measured', () => {
    const { panel, measure, initialProps, rerender } = setup();
    runFrame();
    panel.resize.mockClear();

    measure.mockReturnValue(undefined);
    rerender({ ...initialProps, collapsed: true });
    runFrame();

    expect(panel.resize).not.toHaveBeenCalled();
  });
});

describe('useCollapsiblePanel layout retries', () => {
  it('tries again on the next frame while the panel is not ready', () => {
    const { panel } = setup();
    let ready = false;
    panel.resize.mockImplementation(() => {
      if (!ready) {
        throw new Error('panel not measured');
      }
    });

    runFrame();
    expect(frames.size).toBeGreaterThan(0);
    ready = true;
    runFrame();

    expect(panel.resize).toHaveBeenLastCalledWith(180);
    expect(frames.size).toBe(0);
  });

  it('gives up with the original error after too many attempts', () => {
    const { panel } = setup();
    panel.resize.mockImplementation(() => {
      throw new Error('panel not measured');
    });

    expect(() => {
      for (let frame = 0; frame < 100; frame += 1) {
        runFrame();
      }
    }).toThrow('panel not measured');
  });

  it('stops trying when unmounted', () => {
    const { panel, unmount } = setup();

    unmount();
    runFrame();

    expect(panel.resize).not.toHaveBeenCalled();
  });
});
