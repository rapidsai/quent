// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Provider as JotaiProvider, createStore } from 'jotai';
import { beforeEach, describe, expect, it } from 'vitest';
import { render } from '@/test/test-utils';
import { expandedIdsAtom, pinnedCollapsedAtom, pinnedPanelSizeAtom } from '@/atoms/resourceTree';
import { DeepLinkBoundary } from './DeepLinkBoundary';
import { LocalDeepLinkStorage } from './LocalDeepLinkStorage';
import { decodeDeepLinkState, encodeDeepLinkState } from './deepLink.codec';
import { readDeepLinkFromLocalStorage, writeDeepLinkToLocalStorage } from './deepLink.localStorage';
import { useLocalDeepLinkState } from './useLocalDeepLinkState';

const SCOPE = { engineId: 'e', queryId: 'q' };
const PROPS = {
  ...SCOPE,
  activeTab: 'timeline' as const,
  initialPlanId: '',
  durationSeconds: 100,
  isQueryReady: true,
  operators: [],
};

function createStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: key => values.get(key) ?? null,
    key: index => [...values.keys()][index] ?? null,
    removeItem: key => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
}

function encodedState(expandedRowIds = ['row-1']): string {
  const encoded = encodeDeepLinkState({
    route: { ...SCOPE, tab: 'timeline' },
    timeline: { zoomRange: { start: 10, end: 20 } },
    resources: {
      expandedRowIds,
      pinnedRowsCollapsed: true,
      pinnedRowsPanelSize: 240,
    },
  });
  if (!encoded.ok) {
    throw new Error('fixture failed to encode');
  }
  return encoded.value;
}

function StorageHarness({ urlState }: { urlState?: string }) {
  const localState = useLocalDeepLinkState({
    ...SCOPE,
    activeTab: 'timeline',
    hasUrlState: urlState !== undefined,
    isReady: true,
  });
  const resolvedState = urlState ?? localState ?? undefined;

  return (
    <DeepLinkBoundary {...PROPS} encodedState={resolvedState}>
      <LocalDeepLinkStorage
        {...SCOPE}
        tab="timeline"
        restoring={urlState === undefined && localState !== null}
      />
      <div>content</div>
    </DeepLinkBoundary>
  );
}

describe('deep-link storage mechanisms', () => {
  let storage: Storage;

  beforeEach(() => {
    storage = createStorage();
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: storage,
    });
  });

  it('retrieves complete encoded state from local storage', () => {
    writeDeepLinkToLocalStorage(storage, SCOPE, encodedState());
    const store = createStore();

    render(
      <JotaiProvider store={store}>
        <StorageHarness />
      </JotaiProvider>
    );

    expect([...store.get(expandedIdsAtom)]).toEqual(['row-1']);
    expect(store.get(pinnedCollapsedAtom)).toBe(true);
    expect(store.get(pinnedPanelSizeAtom)).toBe(240);
  });

  it('gives URL state priority over local storage', () => {
    writeDeepLinkToLocalStorage(storage, SCOPE, encodedState());
    const store = createStore();

    render(
      <JotaiProvider store={store}>
        <StorageHarness urlState={encodedState(['row-from-url'])} />
      </JotaiProvider>
    );

    expect([...store.get(expandedIdsAtom)]).toEqual(['row-from-url']);
  });

  it('drops invalid local state before passing it to the boundary', async () => {
    writeDeepLinkToLocalStorage(storage, SCOPE, 'v3.garbage');

    render(
      <JotaiProvider store={createStore()}>
        <StorageHarness />
      </JotaiProvider>
    );

    await Promise.resolve();
    expect(readDeepLinkFromLocalStorage(storage, SCOPE)).toBeNull();
  });

  it('persists encoded state to local storage when the page closes', () => {
    const store = createStore();
    store.set(expandedIdsAtom, new Set(['row-2']));
    store.set(pinnedCollapsedAtom, true);
    store.set(pinnedPanelSizeAtom, 180.4);

    render(
      <JotaiProvider store={store}>
        <DeepLinkBoundary {...PROPS}>
          <LocalDeepLinkStorage {...SCOPE} tab="timeline" restoring={false} />
          <div>content</div>
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(readDeepLinkFromLocalStorage(storage, SCOPE)).toBeNull();
    window.dispatchEvent(new Event('pagehide'));
    const stored = readDeepLinkFromLocalStorage(storage, SCOPE);
    expect(stored).not.toBeNull();
    expect(decodeDeepLinkState(stored!)).toMatchObject({
      ok: true,
      value: {
        data: {
          route: { tab: 'timeline' },
          resources: {
            expandedRowIds: ['row-2'],
            pinnedRowsCollapsed: true,
            pinnedRowsPanelSize: 180,
          },
        },
      },
    });
  });
});
