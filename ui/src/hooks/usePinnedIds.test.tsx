// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { createStore, Provider } from 'jotai';
import { describe, expect, it } from 'vitest';
import { MAX_PINNED_ROWS } from '@/features/pinned-rows/pinnedRows';
import { usePinnedCollapsed, usePinnedIds } from './usePinnedIds';

function renderPinnedHook() {
  const store = createStore();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );
  return renderHook(() => usePinnedIds(), { wrapper });
}

describe('usePinnedIds', () => {
  it('starts with no pinned rows', () => {
    const { result } = renderPinnedHook();

    expect(result.current.pinnedIds.size).toBe(0);
    expect(result.current.isAtPinLimit).toBe(false);
  });

  it('pins a row and unpins it when toggled again', () => {
    const { result } = renderPinnedHook();

    act(() => result.current.togglePin('gpu-0'));
    expect(result.current.pinnedIds.has('gpu-0')).toBe(true);

    act(() => result.current.togglePin('gpu-0'));
    expect(result.current.pinnedIds.has('gpu-0')).toBe(false);
  });

  it('returns a new set on each change instead of mutating the old one', () => {
    const { result } = renderPinnedHook();
    const before = result.current.pinnedIds;

    act(() => result.current.togglePin('gpu-0'));

    expect(result.current.pinnedIds).not.toBe(before);
    expect(before.size).toBe(0);
  });

  it('ignores new pins once the limit is reached', () => {
    const { result } = renderPinnedHook();

    for (let i = 0; i < MAX_PINNED_ROWS; i += 1) {
      act(() => result.current.togglePin(`row-${i}`));
    }
    expect(result.current.pinnedIds.size).toBe(MAX_PINNED_ROWS);
    expect(result.current.isAtPinLimit).toBe(true);

    act(() => result.current.togglePin('one-too-many'));

    expect(result.current.pinnedIds.size).toBe(MAX_PINNED_ROWS);
    expect(result.current.pinnedIds.has('one-too-many')).toBe(false);
  });

  it('still allows unpinning when at the limit', () => {
    const { result } = renderPinnedHook();
    for (let i = 0; i < MAX_PINNED_ROWS; i += 1) {
      act(() => result.current.togglePin(`row-${i}`));
    }

    act(() => result.current.togglePin('row-0'));

    expect(result.current.pinnedIds.has('row-0')).toBe(false);
    expect(result.current.isAtPinLimit).toBe(false);
  });
});

describe('usePinnedIds reordering', () => {
  it('keeps pins in the order they were added', () => {
    const { result } = renderPinnedHook();

    act(() => result.current.togglePin('b'));
    act(() => result.current.togglePin('a'));

    expect([...result.current.pinnedIds]).toEqual(['b', 'a']);
  });

  it('moves a pin to where another pin sits', () => {
    const { result } = renderPinnedHook();
    for (const id of ['a', 'b', 'c']) {
      act(() => result.current.togglePin(id));
    }

    act(() => result.current.movePin('a', 'c'));

    expect([...result.current.pinnedIds]).toEqual(['b', 'c', 'a']);
  });

  it('returns a new set after a move instead of mutating the old one', () => {
    const { result } = renderPinnedHook();
    for (const id of ['a', 'b']) {
      act(() => result.current.togglePin(id));
    }
    const before = result.current.pinnedIds;

    act(() => result.current.movePin('a', 'b'));

    expect(result.current.pinnedIds).not.toBe(before);
    expect([...before]).toEqual(['a', 'b']);
  });
});

describe('usePinnedCollapsed', () => {
  function renderCollapsedHook() {
    const store = createStore();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <Provider store={store}>{children}</Provider>
    );
    return renderHook(() => usePinnedCollapsed(), { wrapper });
  }

  it('starts expanded', () => {
    const { result } = renderCollapsedHook();

    expect(result.current.isPinnedCollapsed).toBe(false);
  });

  it('flips between collapsed and expanded', () => {
    const { result } = renderCollapsedHook();

    act(() => result.current.togglePinnedCollapsed());
    expect(result.current.isPinnedCollapsed).toBe(true);

    act(() => result.current.togglePinnedCollapsed());
    expect(result.current.isPinnedCollapsed).toBe(false);
  });
});
