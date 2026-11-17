// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  VIEW_AUTOSAVE_VERSION,
  ViewAutosaveRecordSchema,
  type ViewAutosaveRecord,
} from './viewAutosave.schema';

const STORAGE_PREFIX = 'quent.viewAutosave.v1';

export interface ViewAutosaveScope {
  engineId: string;
  queryId: string;
}

export type ViewAutosaveSnapshot = Pick<ViewAutosaveRecord, 'tab' | 'payload' | 'extras'>;

export function viewAutosaveStorageKey({ engineId, queryId }: ViewAutosaveScope): string {
  return `${STORAGE_PREFIX}:${encodeURIComponent(engineId)}:${encodeURIComponent(queryId)}`;
}

/** Returns the browser's localStorage, or null when access is blocked. */
export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/**
 * Reads the saved view for a query. Anything missing, unreadable, or from a
 * different format or query returns null; a bad record is never an error.
 */
export function readViewAutosave(
  storage: Storage | null,
  scope: ViewAutosaveScope
): ViewAutosaveRecord | null {
  if (!storage) {
    return null;
  }
  try {
    const raw = storage.getItem(viewAutosaveStorageKey(scope));
    if (raw === null) {
      return null;
    }
    const parsed = ViewAutosaveRecordSchema.safeParse(JSON.parse(raw));
    if (
      !parsed.success ||
      parsed.data.version !== VIEW_AUTOSAVE_VERSION ||
      parsed.data.engineId !== scope.engineId ||
      parsed.data.queryId !== scope.queryId
    ) {
      return null;
    }
    return parsed.data;
  } catch {
    return null;
  }
}

/** Returns true when the view was written. Storage failures are swallowed. */
export function writeViewAutosave(
  storage: Storage | null,
  scope: ViewAutosaveScope,
  snapshot: ViewAutosaveSnapshot,
  now: number = Date.now()
): boolean {
  if (!storage) {
    return false;
  }
  const record: ViewAutosaveRecord = {
    schema: 'quent.view-autosave',
    version: VIEW_AUTOSAVE_VERSION,
    engineId: scope.engineId,
    queryId: scope.queryId,
    updatedAt: now,
    ...snapshot,
  };
  try {
    storage.setItem(viewAutosaveStorageKey(scope), JSON.stringify(record));
    return true;
  } catch {
    // Storage is full or blocked; losing the remembered view is acceptable.
    return false;
  }
}

export function clearViewAutosave(storage: Storage | null, scope: ViewAutosaveScope): void {
  try {
    storage?.removeItem(viewAutosaveStorageKey(scope));
  } catch {
    // Nothing to clear if storage is blocked.
  }
}
