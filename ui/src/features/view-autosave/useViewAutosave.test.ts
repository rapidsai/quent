// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useViewAutosave } from './useViewAutosave';
import { readViewAutosave } from './viewAutosave.storage';
import { useRestoredView } from './useRestoredView';
import { writeViewAutosave } from './viewAutosave.storage';

const SCOPE = { engineId: 'e', queryId: 'q' };
const snapshot = (tab: 'timeline' | 'operators', payload = 'v3.aaa') => ({ tab, payload });

describe('useViewAutosave', () => {
  beforeEach(() => localStorage.clear());

  it('does not write while the user interacts', () => {
    const capture = vi.fn(() => snapshot('timeline'));
    renderHook(() => useViewAutosave({ scope: SCOPE, tab: 'timeline', enabled: true, capture }));
    expect(capture).not.toHaveBeenCalled();
    expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
  });

  it('saves on pagehide', () => {
    renderHook(() =>
      useViewAutosave({
        scope: SCOPE,
        tab: 'timeline',
        enabled: true,
        capture: () => snapshot('timeline'),
      })
    );
    window.dispatchEvent(new Event('pagehide'));
    expect(readViewAutosave(localStorage, SCOPE)?.payload).toBe('v3.aaa');
  });

  it('saves when the page becomes hidden', () => {
    renderHook(() =>
      useViewAutosave({
        scope: SCOPE,
        tab: 'timeline',
        enabled: true,
        capture: () => snapshot('timeline'),
      })
    );
    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
    document.dispatchEvent(new Event('visibilitychange'));
    expect(readViewAutosave(localStorage, SCOPE)).not.toBeNull();
    vi.restoreAllMocks();
  });

  it('saves on unmount', () => {
    const { unmount } = renderHook(() =>
      useViewAutosave({
        scope: SCOPE,
        tab: 'timeline',
        enabled: true,
        capture: () => snapshot('timeline'),
      })
    );
    unmount();
    expect(readViewAutosave(localStorage, SCOPE)).not.toBeNull();
  });

  it('saves the tab being left when the tab changes', () => {
    const { rerender } = renderHook(
      ({ tab }: { tab: 'timeline' | 'operators' }) =>
        useViewAutosave({
          scope: SCOPE,
          tab,
          enabled: true,
          capture: () => snapshot(tab, tab === 'timeline' ? 'v3.old' : 'v3.new'),
        }),
      { initialProps: { tab: 'timeline' as 'timeline' | 'operators' } }
    );
    rerender({ tab: 'operators' });
    expect(readViewAutosave(localStorage, SCOPE)).toMatchObject({
      tab: 'timeline',
      payload: 'v3.old',
    });
  });

  it('writes nothing while disabled', () => {
    const capture = vi.fn(() => snapshot('timeline'));
    const { unmount } = renderHook(() =>
      useViewAutosave({ scope: SCOPE, tab: 'timeline', enabled: false, capture })
    );
    window.dispatchEvent(new Event('pagehide'));
    unmount();
    expect(capture).not.toHaveBeenCalled();
  });

  it('keeps the old view when capture is unavailable', () => {
    writeViewAutosave(localStorage, SCOPE, snapshot('timeline', 'v3.keep'));
    const { unmount } = renderHook(() =>
      useViewAutosave({ scope: SCOPE, tab: 'timeline', enabled: true, capture: () => null })
    );
    unmount();
    expect(readViewAutosave(localStorage, SCOPE)?.payload).toBe('v3.keep');
  });
});

describe('useRestoredView', () => {
  beforeEach(() => localStorage.clear());
  const base = {
    engineId: 'e',
    queryId: 'q',
    activeTab: 'timeline' as const,
    hasUrlState: false,
    isReady: true,
  };

  it('returns the saved view for the matching tab', () => {
    writeViewAutosave(localStorage, SCOPE, snapshot('timeline'));
    const { result } = renderHook(() => useRestoredView(base));
    expect(result.current?.payload).toBe('v3.aaa');
  });

  it('ignores a view saved on another tab', () => {
    writeViewAutosave(localStorage, SCOPE, snapshot('operators'));
    const { result } = renderHook(() => useRestoredView(base));
    expect(result.current).toBeNull();
  });

  it('lets a shared link win', () => {
    writeViewAutosave(localStorage, SCOPE, snapshot('timeline'));
    const { result } = renderHook(() => useRestoredView({ ...base, hasUrlState: true }));
    expect(result.current).toBeNull();
  });

  it('does not restore again after a later save', () => {
    const { result, rerender } = renderHook(() => useRestoredView(base));
    writeViewAutosave(localStorage, SCOPE, snapshot('timeline'));
    rerender();
    expect(result.current).toBeNull();
  });
});
