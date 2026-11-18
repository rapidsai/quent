// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { MAX_ENCODED_STATE_LENGTH } from './viewState.schema';
import { decodeViewState } from './viewState.codec';
import { readViewStateFields } from './viewState.fields';
import type { ViewStateScope, EncodedViewStateRepository } from './viewState.storage';

const STORAGE_PREFIX = 'quent.viewState.v1'; // Preserve existing browser state.

export function viewStateStorageKey({ engineId, queryId }: ViewStateScope): string {
  return `${STORAGE_PREFIX}:${encodeURIComponent(engineId)}:${encodeURIComponent(queryId)}`;
}

export function browserLocalStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readViewStateFromLocalStorage(
  storage: Storage | null,
  scope: ViewStateScope
): string | null {
  if (!storage) {
    return null;
  }
  try {
    const value = storage.getItem(viewStateStorageKey(scope));
    if (
      value === null ||
      value.length > MAX_ENCODED_STATE_LENGTH ||
      !/^v\d+\.[A-Za-z0-9_-]+$/u.test(value)
    ) {
      return null;
    }
    return value;
  } catch {
    return null;
  }
}

export function writeViewStateToLocalStorage(
  storage: Storage | null,
  scope: ViewStateScope,
  encodedState: string
): boolean {
  if (!storage) {
    return false;
  }
  try {
    storage.setItem(viewStateStorageKey(scope), encodedState);
    return true;
  } catch {
    return false;
  }
}

export function clearViewStateFromLocalStorage(
  storage: Storage | null,
  scope: ViewStateScope
): void {
  try {
    storage?.removeItem(viewStateStorageKey(scope));
  } catch {
    // Blocked storage is already effectively empty.
  }
}

export function createLocalViewStateRepository(
  getStorage: () => Storage | null = browserLocalStorage
): EncodedViewStateRepository {
  return {
    id: 'local-storage',
    load(scope) {
      const encodedState = readViewStateFromLocalStorage(getStorage(), scope);
      if (!encodedState) {
        return null;
      }
      const decoded = decodeViewState(encodedState);
      const route = decoded.ok ? readViewStateFields(decoded.value).route : undefined;
      if (!route || route.engineId !== scope.engineId || route.queryId !== scope.queryId) {
        clearViewStateFromLocalStorage(getStorage(), scope);
        return null;
      }
      return scope.tab === undefined || route.tab === scope.tab ? encodedState : null;
    },
    save(scope, encodedState) {
      writeViewStateToLocalStorage(getStorage(), scope, encodedState);
    },
    remove(scope) {
      clearViewStateFromLocalStorage(getStorage(), scope);
    },
  };
}
