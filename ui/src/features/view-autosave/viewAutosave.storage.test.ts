// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vitest';
import {
  clearViewAutosave,
  readViewAutosave,
  viewAutosaveStorageKey,
  writeViewAutosave,
} from './viewAutosave.storage';

const SCOPE = { engineId: 'e', queryId: 'q' };
const SNAPSHOT = { tab: 'timeline' as const, payload: 'v3.abc_-9' };

describe('view autosave storage', () => {
  beforeEach(() => localStorage.clear());

  it('round-trips a view', () => {
    expect(
      writeViewAutosave(localStorage, SCOPE, { ...SNAPSHOT, extras: { pinnedCollapsed: true } })
    ).toBe(true);
    expect(readViewAutosave(localStorage, SCOPE)).toMatchObject({
      ...SNAPSHOT,
      extras: { pinnedCollapsed: true },
    });
  });

  it('keeps queries separate', () => {
    writeViewAutosave(localStorage, SCOPE, SNAPSHOT);
    expect(readViewAutosave(localStorage, { engineId: 'e', queryId: 'other' })).toBeNull();
  });

  it.each([
    ['invalid json', '{nope'],
    ['wrong shape', JSON.stringify({ schema: 'x' })],
    ['future version', JSON.stringify({ schema: 'quent.view-autosave', version: 2 })],
  ])('treats %s as no saved view', (_name, raw) => {
    localStorage.setItem(viewAutosaveStorageKey(SCOPE), raw);
    expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
  });

  it('ignores a record stored under the wrong query', () => {
    writeViewAutosave(localStorage, { engineId: 'e', queryId: 'other' }, SNAPSHOT);
    localStorage.setItem(
      viewAutosaveStorageKey(SCOPE),
      localStorage.getItem(viewAutosaveStorageKey({ engineId: 'e', queryId: 'other' })) ?? ''
    );
    expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
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
    expect(readViewAutosave(broken, SCOPE)).toBeNull();
    expect(writeViewAutosave(broken, SCOPE, SNAPSHOT)).toBe(false);
    expect(() => clearViewAutosave(broken, SCOPE)).not.toThrow();
    expect(writeViewAutosave(null, SCOPE, SNAPSHOT)).toBe(false);
  });

  it('clears a saved view', () => {
    writeViewAutosave(localStorage, SCOPE, SNAPSHOT);
    clearViewAutosave(localStorage, SCOPE);
    expect(readViewAutosave(localStorage, SCOPE)).toBeNull();
  });
});
