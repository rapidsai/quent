// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Provider as JotaiProvider, createStore } from 'jotai';
import { beforeEach, describe, expect, it } from 'vitest';
import { render } from '@/test/test-utils';
import { expandedIdsAtom, pinnedCollapsedAtom, pinnedPanelSizeAtom } from '@/atoms/resourceTree';
import { readViewAutosave, writeViewAutosave } from '@/features/view-autosave';
import { DeepLinkBoundary } from './DeepLinkBoundary';
import { encodeDeepLinkState } from './deepLink.codec';

const SCOPE = { engineId: 'e', queryId: 'q' };
const PROPS = {
  engineId: 'e',
  queryId: 'q',
  activeTab: 'timeline' as const,
  initialPlanId: '',
  durationSeconds: 100,
  isQueryReady: true,
  operators: [],
};

function storedPayload(): string {
  const encoded = encodeDeepLinkState({
    route: { engineId: 'e', queryId: 'q', tab: 'timeline' },
    timeline: { zoomRange: { start: 10, end: 20 } },
    resources: { expandedRowIds: ['row-1'] },
  });
  if (!encoded.ok) {
    throw new Error('fixture failed to encode');
  }
  return encoded.value;
}

describe('DeepLinkBoundary autosave', () => {
  beforeEach(() => localStorage.clear());

  it('restores a stored view, including extras, without a toast', () => {
    const store = createStore();
    render(
      <JotaiProvider store={store}>
        <DeepLinkBoundary
          {...PROPS}
          encodedState={storedPayload()}
          restoreSource="stored"
          storedExtras={{ pinnedCollapsed: true, pinnedPanelSize: 240 }}
        >
          <div>content</div>
        </DeepLinkBoundary>
      </JotaiProvider>
    );
    expect([...store.get(expandedIdsAtom)]).toEqual(['row-1']);
    expect(store.get(pinnedCollapsedAtom)).toBe(true);
    expect(store.get(pinnedPanelSizeAtom)).toBe(240);
  });

  it('drops a stored view that no longer fits', () => {
    writeViewAutosave(localStorage, SCOPE, { tab: 'timeline', payload: 'v3.garbage' });
    render(
      <JotaiProvider store={createStore()}>
        <DeepLinkBoundary {...PROPS} encodedState="v3.garbage" restoreSource="stored">
          <div>content</div>
        </DeepLinkBoundary>
      </JotaiProvider>
    );
    return Promise.resolve().then(() => {
      expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
    });
  });

  it('writes the current view when the page closes', () => {
    const store = createStore();
    store.set(expandedIdsAtom, new Set(['row-2']));
    store.set(pinnedCollapsedAtom, true);
    store.set(pinnedPanelSizeAtom, 180.4);
    render(
      <JotaiProvider store={store}>
        <DeepLinkBoundary {...PROPS}>
          <div>content</div>
        </DeepLinkBoundary>
      </JotaiProvider>
    );
    expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
    window.dispatchEvent(new Event('pagehide'));
    const saved = readViewAutosave(localStorage, SCOPE);
    expect(saved?.tab).toBe('timeline');
    expect(saved?.extras).toEqual({ pinnedCollapsed: true, pinnedPanelSize: 180 });
  });
});
