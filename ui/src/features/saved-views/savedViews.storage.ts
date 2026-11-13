// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  MAX_SAVED_VIEWS_PER_QUERY,
  SAVED_VIEWS_SCHEMA,
  SAVED_VIEWS_VERSION,
  SavedViewV1Schema,
  SavedViewsDocumentHeaderV1Schema,
  type SavedViewV1,
  type SavedViewsDocumentV1,
} from './savedViews.schema';

const SAVED_VIEWS_STORAGE_PREFIX = 'quent.savedViews.v1';

export interface SavedViewsScope {
  engineId: string;
  queryId: string;
}

export interface SavedViewsSnapshot {
  views: SavedViewV1[];
  invalidEntries: unknown[];
}

export type SavedViewsReadResult =
  | { ok: true; value: SavedViewsSnapshot }
  | { ok: false; kind: 'unavailable' | 'corrupt' | 'future-version'; message: string };

export type SavedViewsWriteResult =
  | { ok: true }
  | {
      ok: false;
      kind: 'unavailable' | 'corrupt' | 'future-version' | 'quota' | 'limit';
      message: string;
    };

export function savedViewsStorageKey({ engineId, queryId }: SavedViewsScope): string {
  return `${SAVED_VIEWS_STORAGE_PREFIX}:${encodeURIComponent(engineId)}:${encodeURIComponent(queryId)}`;
}

function emptySnapshot(): SavedViewsSnapshot {
  return { views: [], invalidEntries: [] };
}

function storageUnavailable(): SavedViewsReadResult {
  return {
    ok: false,
    kind: 'unavailable',
    message: 'Browser storage is unavailable.',
  };
}

export function readSavedViews(storage: Storage, scope: SavedViewsScope): SavedViewsReadResult {
  let raw: string | null;
  try {
    raw = storage.getItem(savedViewsStorageKey(scope));
  } catch {
    return storageUnavailable();
  }
  if (raw === null) {
    return { ok: true, value: emptySnapshot() };
  }

  let parsedJson: unknown;
  try {
    parsedJson = JSON.parse(raw) as unknown;
  } catch {
    return {
      ok: false,
      kind: 'corrupt',
      message: 'Saved views are unreadable.',
    };
  }

  const parsedHeader = SavedViewsDocumentHeaderV1Schema.safeParse(parsedJson);
  if (!parsedHeader.success) {
    return {
      ok: false,
      kind: 'corrupt',
      message: 'Saved views do not match the expected format.',
    };
  }
  const document = parsedHeader.data;
  if (document.version > SAVED_VIEWS_VERSION) {
    return {
      ok: false,
      kind: 'future-version',
      message: 'Saved views were created by a newer Quent version.',
    };
  }
  if (document.version < SAVED_VIEWS_VERSION) {
    return {
      ok: false,
      kind: 'corrupt',
      message: 'Saved views use an older unsupported format.',
    };
  }
  if (document.engineId !== scope.engineId || document.queryId !== scope.queryId) {
    return {
      ok: false,
      kind: 'corrupt',
      message: 'Saved views do not match this engine and query.',
    };
  }

  const views: SavedViewV1[] = [];
  const invalidEntries: unknown[] = [];
  for (const entry of document.views) {
    const parsedEntry = SavedViewV1Schema.safeParse(entry);
    if (parsedEntry.success) {
      views.push(parsedEntry.data);
    } else {
      invalidEntries.push(entry);
    }
  }
  return { ok: true, value: { views, invalidEntries } };
}

function isQuotaError(error: unknown): boolean {
  return (
    error instanceof DOMException &&
    (error.name === 'QuotaExceededError' ||
      error.name === 'NS_ERROR_DOM_QUOTA_REACHED' ||
      error.code === 22)
  );
}

export function writeSavedViews(
  storage: Storage,
  scope: SavedViewsScope,
  snapshot: SavedViewsSnapshot
): SavedViewsWriteResult {
  if (snapshot.views.length > MAX_SAVED_VIEWS_PER_QUERY) {
    return {
      ok: false,
      kind: 'limit',
      message: `Saved views support at most ${MAX_SAVED_VIEWS_PER_QUERY} views per query.`,
    };
  }

  const parsedViews = snapshot.views.map(view => SavedViewV1Schema.safeParse(view));
  if (parsedViews.some(result => !result.success)) {
    return {
      ok: false,
      kind: 'corrupt',
      message: 'A saved view does not match the expected format.',
    };
  }

  const document: SavedViewsDocumentV1 = {
    schema: SAVED_VIEWS_SCHEMA,
    version: SAVED_VIEWS_VERSION,
    engineId: scope.engineId,
    queryId: scope.queryId,
    views: [
      ...parsedViews.map(result => {
        if (!result.success) {
          throw new Error('Validated saved view unexpectedly failed parsing.');
        }
        return result.data;
      }),
      ...snapshot.invalidEntries,
    ],
  };

  try {
    storage.setItem(savedViewsStorageKey(scope), JSON.stringify(document));
    return { ok: true };
  } catch (error) {
    return isQuotaError(error)
      ? {
          ok: false,
          kind: 'quota',
          message: 'Browser storage is full. Delete saved views and try again.',
        }
      : {
          ok: false,
          kind: 'unavailable',
          message: 'Browser storage is unavailable.',
        };
  }
}

export function resetSavedViews(storage: Storage, scope: SavedViewsScope): SavedViewsWriteResult {
  try {
    storage.removeItem(savedViewsStorageKey(scope));
    return { ok: true };
  } catch {
    return {
      ok: false,
      kind: 'unavailable',
      message: 'Browser storage is unavailable.',
    };
  }
}
