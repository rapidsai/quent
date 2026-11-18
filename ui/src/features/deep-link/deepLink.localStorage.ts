// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { MAX_ENCODED_STATE_LENGTH } from './deepLink.schema';

const STORAGE_PREFIX = 'quent.deepLink.v1';

export interface DeepLinkStorageScope {
  engineId: string;
  queryId: string;
}

export function deepLinkStorageKey({ engineId, queryId }: DeepLinkStorageScope): string {
  return `${STORAGE_PREFIX}:${encodeURIComponent(engineId)}:${encodeURIComponent(queryId)}`;
}

export function browserLocalStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readDeepLinkFromLocalStorage(
  storage: Storage | null,
  scope: DeepLinkStorageScope
): string | null {
  if (!storage) {
    return null;
  }
  try {
    const value = storage.getItem(deepLinkStorageKey(scope));
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

export function writeDeepLinkToLocalStorage(
  storage: Storage | null,
  scope: DeepLinkStorageScope,
  encodedState: string
): boolean {
  if (!storage) {
    return false;
  }
  try {
    storage.setItem(deepLinkStorageKey(scope), encodedState);
    return true;
  } catch {
    return false;
  }
}

export function clearDeepLinkFromLocalStorage(
  storage: Storage | null,
  scope: DeepLinkStorageScope
): void {
  try {
    storage?.removeItem(deepLinkStorageKey(scope));
  } catch {
    // Blocked storage is already effectively empty.
  }
}
