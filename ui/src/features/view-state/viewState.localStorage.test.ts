// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vitest';
import {
  clearViewStateFromLocalStorage,
  viewStateStorageKey,
  readViewStateFromLocalStorage,
  writeViewStateToLocalStorage,
} from './viewState.localStorage';

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

describe('view-state local storage', () => {
  let storage: Storage;

  beforeEach(() => {
    storage = createStorage();
  });

  it('round-trips encoded view state', () => {
    expect(writeViewStateToLocalStorage(storage, SCOPE, STATE)).toBe(true);
    expect(readViewStateFromLocalStorage(storage, SCOPE)).toBe(STATE);
  });

  it('keeps queries separate', () => {
    writeViewStateToLocalStorage(storage, SCOPE, STATE);
    expect(readViewStateFromLocalStorage(storage, { engineId: 'e', queryId: 'other' })).toBeNull();
  });

  it.each(['{nope', 'plain text', 'v3.abc.123'])('treats invalid state %s as missing', raw => {
    storage.setItem(viewStateStorageKey(SCOPE), raw);
    expect(readViewStateFromLocalStorage(storage, SCOPE)).toBeNull();
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

    expect(readViewStateFromLocalStorage(broken, SCOPE)).toBeNull();
    expect(writeViewStateToLocalStorage(broken, SCOPE, STATE)).toBe(false);
    expect(() => clearViewStateFromLocalStorage(broken, SCOPE)).not.toThrow();
    expect(writeViewStateToLocalStorage(null, SCOPE, STATE)).toBe(false);
  });

  it('clears stored state', () => {
    writeViewStateToLocalStorage(storage, SCOPE, STATE);
    clearViewStateFromLocalStorage(storage, SCOPE);
    expect(readViewStateFromLocalStorage(storage, SCOPE)).toBeNull();
  });
});
