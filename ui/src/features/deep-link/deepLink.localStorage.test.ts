// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vitest';
import {
  clearDeepLinkFromLocalStorage,
  deepLinkStorageKey,
  readDeepLinkFromLocalStorage,
  writeDeepLinkToLocalStorage,
} from './deepLink.localStorage';

const SCOPE = { engineId: 'e', queryId: 'q' };
const STATE = 'v3.abc_-9';

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

describe('deep-link local storage', () => {
  let storage: Storage;

  beforeEach(() => {
    storage = createStorage();
  });

  it('round-trips encoded deep-link state', () => {
    expect(writeDeepLinkToLocalStorage(storage, SCOPE, STATE)).toBe(true);
    expect(readDeepLinkFromLocalStorage(storage, SCOPE)).toBe(STATE);
  });

  it('keeps queries separate', () => {
    writeDeepLinkToLocalStorage(storage, SCOPE, STATE);
    expect(readDeepLinkFromLocalStorage(storage, { engineId: 'e', queryId: 'other' })).toBeNull();
  });

  it.each(['{nope', 'plain text', 'v3.abc.123'])('treats invalid state %s as missing', raw => {
    storage.setItem(deepLinkStorageKey(SCOPE), raw);
    expect(readDeepLinkFromLocalStorage(storage, SCOPE)).toBeNull();
  });

  it('does not throw when storage is blocked or full', () => {
    const broken = {
      getItem: () => {
        throw new DOMException('blocked', 'SecurityError');
      },
      setItem: () => {
        throw new DOMException('full', 'QuotaExceededError');
      },
      removeItem: () => {
        throw new DOMException('blocked', 'SecurityError');
      },
    } as unknown as Storage;

    expect(readDeepLinkFromLocalStorage(broken, SCOPE)).toBeNull();
    expect(writeDeepLinkToLocalStorage(broken, SCOPE, STATE)).toBe(false);
    expect(() => clearDeepLinkFromLocalStorage(broken, SCOPE)).not.toThrow();
    expect(writeDeepLinkToLocalStorage(null, SCOPE, STATE)).toBe(false);
  });

  it('clears stored state', () => {
    writeDeepLinkToLocalStorage(storage, SCOPE, STATE);
    clearDeepLinkFromLocalStorage(storage, SCOPE);
    expect(readDeepLinkFromLocalStorage(storage, SCOPE)).toBeNull();
  });
});
