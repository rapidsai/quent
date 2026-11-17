// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useSyncExternalStore } from 'react';
import {
  readSavedViews,
  resetSavedViews,
  savedViewsStorageKey,
  writeSavedViews,
  type SavedViewsReadResult,
  type SavedViewsScope,
  type SavedViewsWriteResult,
} from './savedViews.storage';
import type { SavedViewExtrasV1, SavedViewV1 } from './savedViews.schema';
import type { DeepLinkTab } from '@/features/deep-link/deepLink.schema';

export interface NewSavedView {
  name: string;
  tab: DeepLinkTab;
  payload: string;
  extras?: SavedViewExtrasV1;
}

export type SavedViewsSaveResult =
  Exclude<SavedViewsWriteResult, { ok: true }> | { ok: true; id: string };

export type SavedViewSnapshot = Omit<NewSavedView, 'name'>;

const unavailableStorage = {
  getItem() {
    throw new DOMException('Storage unavailable', 'SecurityError');
  },
  setItem() {
    throw new DOMException('Storage unavailable', 'SecurityError');
  },
  removeItem() {
    throw new DOMException('Storage unavailable', 'SecurityError');
  },
} as unknown as Storage;

function browserStorage(): Storage {
  try {
    return window.localStorage;
  } catch {
    return unavailableStorage;
  }
}

function readFailureToWrite(
  result: Exclude<SavedViewsReadResult, { ok: true }>
): Exclude<SavedViewsWriteResult, { ok: true }> {
  return result;
}

function createSavedViewsStore(storage: Storage, scope: SavedViewsScope) {
  let snapshot = readSavedViews(storage, scope);
  const listeners = new Set<() => void>();
  const key = savedViewsStorageKey(scope);

  const refresh = () => {
    snapshot = readSavedViews(storage, scope);
    for (const listener of listeners) {
      listener();
    }
  };

  return {
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      const onStorage = (event: StorageEvent) => {
        if (event.storageArea === storage && event.key === key) {
          refresh();
        }
      };
      window.addEventListener('storage', onStorage);
      return () => {
        listeners.delete(listener);
        window.removeEventListener('storage', onStorage);
      };
    },
    save(input: NewSavedView): SavedViewsSaveResult {
      const current = readSavedViews(storage, scope);
      if (!current.ok) {
        return readFailureToWrite(current);
      }
      const now = Date.now();
      const view: SavedViewV1 = {
        id: crypto.randomUUID(),
        name: input.name.trim(),
        createdAt: now,
        updatedAt: now,
        tab: input.tab,
        payload: input.payload,
        ...(input.extras ? { extras: input.extras } : {}),
      };
      const result = writeSavedViews(storage, scope, {
        views: [...current.value.views, view],
        invalidEntries: current.value.invalidEntries,
      });
      if (result.ok) {
        refresh();
        return { ok: true, id: view.id };
      }
      return result;
    },
    update(id: string, input: SavedViewSnapshot): SavedViewsWriteResult {
      const current = readSavedViews(storage, scope);
      if (!current.ok) {
        return readFailureToWrite(current);
      }
      const index = current.value.views.findIndex(view => view.id === id);
      if (index === -1) {
        return { ok: false, kind: 'corrupt', message: 'The saved view no longer exists.' };
      }
      const views = [...current.value.views];
      const updatedView: SavedViewV1 = {
        ...views[index],
        tab: input.tab,
        payload: input.payload,
        updatedAt: Date.now(),
      };
      if (input.extras) {
        updatedView.extras = input.extras;
      } else {
        delete updatedView.extras;
      }
      views[index] = updatedView;
      const result = writeSavedViews(storage, scope, {
        views,
        invalidEntries: current.value.invalidEntries,
      });
      if (result.ok) {
        refresh();
      }
      return result;
    },
    rename(id: string, name: string): SavedViewsWriteResult {
      const current = readSavedViews(storage, scope);
      if (!current.ok) {
        return readFailureToWrite(current);
      }
      const index = current.value.views.findIndex(view => view.id === id);
      if (index === -1) {
        return { ok: false, kind: 'corrupt', message: 'The saved view no longer exists.' };
      }
      const views = [...current.value.views];
      views[index] = { ...views[index], name: name.trim(), updatedAt: Date.now() };
      const result = writeSavedViews(storage, scope, {
        views,
        invalidEntries: current.value.invalidEntries,
      });
      if (result.ok) {
        refresh();
      }
      return result;
    },
    remove(id: string): SavedViewsWriteResult {
      const current = readSavedViews(storage, scope);
      if (!current.ok) {
        return readFailureToWrite(current);
      }
      const views = current.value.views.filter(view => view.id !== id);
      if (views.length === current.value.views.length) {
        return { ok: false, kind: 'corrupt', message: 'The saved view no longer exists.' };
      }
      const result = writeSavedViews(storage, scope, {
        views,
        invalidEntries: current.value.invalidEntries,
      });
      if (result.ok) {
        refresh();
      }
      return result;
    },
    reset(): SavedViewsWriteResult {
      const result = resetSavedViews(storage, scope);
      if (result.ok) {
        refresh();
      }
      return result;
    },
  };
}

export function useSavedViews({
  engineId,
  queryId,
  storage = browserStorage(),
}: SavedViewsScope & { storage?: Storage }) {
  const store = useMemo(
    () => createSavedViewsStore(storage, { engineId, queryId }),
    [engineId, queryId, storage]
  );
  const result = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
  return {
    result,
    save: store.save,
    update: store.update,
    rename: store.rename,
    remove: store.remove,
    reset: store.reset,
  };
}
