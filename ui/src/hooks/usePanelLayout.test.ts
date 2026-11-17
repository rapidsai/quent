// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { usePanelLayout } from './usePanelLayout';

describe('usePanelLayout', () => {
  beforeEach(() => localStorage.clear());

  it('has no layout until one is saved', () => {
    const { result } = renderHook(() => usePanelLayout('g', ['a', 'b']));
    expect(result.current.defaultLayout).toBeUndefined();
  });

  it('returns a layout saved by an earlier visit', () => {
    const first = renderHook(() => usePanelLayout('g', ['a', 'b']));
    first.result.current.onLayoutChanged({ a: 30, b: 70 }, { isUserInteraction: true } as never);
    const second = renderHook(() => usePanelLayout('g', ['a', 'b']));
    expect(second.result.current.defaultLayout).toEqual({ a: 30, b: 70 });
  });

  it('keeps groups separate', () => {
    renderHook(() => usePanelLayout('g', ['a', 'b'])).result.current.onLayoutChanged(
      { a: 30, b: 70 },
      { isUserInteraction: true } as never
    );
    expect(
      renderHook(() => usePanelLayout('other', ['a', 'b'])).result.current.defaultLayout
    ).toBeUndefined();
  });
});
