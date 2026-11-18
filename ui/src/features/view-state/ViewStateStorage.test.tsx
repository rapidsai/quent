// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { Provider as JotaiProvider, createStore } from 'jotai';
import { beforeEach, describe, expect, it } from 'vitest';
import { render, waitFor } from '@/test/test-utils';
import { expandedIdsAtom, pinnedCollapsedAtom, pinnedPanelSizeAtom } from '@/atoms/resourceTree';
import { ViewStateBoundary } from './ViewStateBoundary';
import { ViewStatePersistence } from './ViewStatePersistence';
import { decodeViewState, encodeViewState } from './viewState.codec';
import {
  createLocalViewStateRepository,
  readViewStateFromLocalStorage,
} from './viewState.localStorage';
import type { EncodedViewStateRepository } from './viewState.storage';
import { createUrlViewStateSource } from './viewState.url';
import { useResolvedViewState } from './useResolvedViewState';

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
  const encoded = encodeViewState({
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

function StorageHarness({
  repository,
  urlState,
}: {
  repository: EncodedViewStateRepository;
  urlState?: string;
}) {
  const sources = useMemo(
    () => [createUrlViewStateSource(urlState), repository],
    [repository, urlState]
  );
  const resolution = useResolvedViewState({
    enabled: true,
    resolutionKey: `${SCOPE.engineId}:${SCOPE.queryId}:${urlState ?? ''}`,
    scope: { ...SCOPE, tab: 'timeline' },
    sources,
  });
  if (resolution.status !== 'ready') {
    return null;
  }

  return (
    <ViewStateBoundary {...PROPS} encodedState={resolution.encodedState ?? undefined}>
      <ViewStatePersistence
        repository={repository}
        {...SCOPE}
        tab="timeline"
        restoring={resolution.sourceId === repository.id}
      />
      <div>content</div>
    </ViewStateBoundary>
  );
}

describe('view-state storage mechanisms', () => {
  let storage: Storage;
  let repository: EncodedViewStateRepository;

  beforeEach(() => {
    storage = createStorage();
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: storage,
    });
    repository = createLocalViewStateRepository(() => storage);
  });

  it('retrieves complete encoded state from local storage', async () => {
    repository.save(SCOPE, encodedState());
    const store = createStore();

    render(
      <JotaiProvider store={store}>
        <StorageHarness repository={repository} />
      </JotaiProvider>
    );

    await waitFor(() => expect([...store.get(expandedIdsAtom)]).toEqual(['row-1']));
    expect(store.get(pinnedCollapsedAtom)).toBe(true);
    expect(store.get(pinnedPanelSizeAtom)).toBe(240);
  });

  it('gives URL state priority over local storage', async () => {
    repository.save(SCOPE, encodedState());
    const store = createStore();

    render(
      <JotaiProvider store={store}>
        <StorageHarness repository={repository} urlState={encodedState(['row-from-url'])} />
      </JotaiProvider>
    );

    await waitFor(() => expect([...store.get(expandedIdsAtom)]).toEqual(['row-from-url']));
  });

  it('drops invalid local state before passing it to the boundary', async () => {
    repository.save(SCOPE, 'v3.garbage');

    render(
      <JotaiProvider store={createStore()}>
        <StorageHarness repository={repository} />
      </JotaiProvider>
    );

    await waitFor(() => expect(readViewStateFromLocalStorage(storage, SCOPE)).toBeNull());
  });

  it('persists encoded state to local storage when the page closes', () => {
    const store = createStore();
    store.set(expandedIdsAtom, new Set(['row-2']));
    store.set(pinnedCollapsedAtom, true);
    store.set(pinnedPanelSizeAtom, 180.4);

    render(
      <JotaiProvider store={store}>
        <ViewStateBoundary {...PROPS}>
          <ViewStatePersistence
            repository={repository}
            {...SCOPE}
            tab="timeline"
            restoring={false}
          />
          <div>content</div>
        </ViewStateBoundary>
      </JotaiProvider>
    );

    expect(readViewStateFromLocalStorage(storage, SCOPE)).toBeNull();
    window.dispatchEvent(new Event('pagehide'));
    const stored = readViewStateFromLocalStorage(storage, SCOPE);
    expect(stored).not.toBeNull();
    expect(decodeViewState(stored!)).toMatchObject({
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
