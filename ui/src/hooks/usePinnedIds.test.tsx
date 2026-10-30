// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { createStore, Provider } from 'jotai';
import { describe, expect, it } from 'vitest';
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

  it('does not cap how many rows can be pinned', () => {
    const { result } = renderPinnedHook();

    for (let i = 0; i < 150; i += 1) {
      act(() => result.current.togglePin(`row-${i}`));
    }

    expect(result.current.pinnedIds.size).toBe(150);
  });
});

describe('usePinnedIds bulk changes', () => {
  it('pins many rows at once, after the existing pins', () => {
    const { result } = renderPinnedHook();
    act(() => result.current.togglePin('x'));

    act(() => result.current.pinMany(['a', 'b']));

    expect([...result.current.pinnedIds]).toEqual(['x', 'a', 'b']);
  });

  it('does not duplicate rows that are already pinned', () => {
    const { result } = renderPinnedHook();
    act(() => result.current.togglePin('a'));

    act(() => result.current.pinMany(['a', 'b']));

    expect([...result.current.pinnedIds]).toEqual(['a', 'b']);
  });

  it('pins every row it is given, however many', () => {
    const { result } = renderPinnedHook();
    const many = Array.from({ length: 150 }, (_, i) => `row-${i}`);

    act(() => result.current.pinMany(many));

    expect([...result.current.pinnedIds]).toEqual(many);
  });

  it('returns a new set after pinning many instead of mutating the old one', () => {
    const { result } = renderPinnedHook();
    const before = result.current.pinnedIds;

    act(() => result.current.pinMany(['a']));

    expect(result.current.pinnedIds).not.toBe(before);
    expect(before.size).toBe(0);
  });

  it('unpins many rows and leaves the others in order', () => {
    const { result } = renderPinnedHook();
    act(() => result.current.pinMany(['a', 'b', 'c', 'd']));

    act(() => result.current.unpinMany(['b', 'd', 'not-pinned']));

    expect([...result.current.pinnedIds]).toEqual(['a', 'c']);
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
