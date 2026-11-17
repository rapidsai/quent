// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { decodeDeepLinkState } from '@/features/deep-link/deepLink.codec';
import type { CaptureSavedViewResult } from '@/features/deep-link/deepLink.context';
import type { SavedViewV1 } from './savedViews.schema';

// Sorts object keys so equal extras always produce the same string.
function stableStringify(value: unknown): string {
  return JSON.stringify(value ?? {}, (_key, v: unknown) =>
    v && typeof v === 'object' && !Array.isArray(v)
      ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => a.localeCompare(b)))
      : v
  );
}

// Compressed payloads can differ in bytes for the same state, so compare decoded state.
function samePayload(a: string, b: string): boolean {
  if (a === b) {
    return true;
  }
  const decodedA = decodeDeepLinkState(a);
  const decodedB = decodeDeepLinkState(b);
  return (
    decodedA.ok &&
    decodedB.ok &&
    decodedA.value.version === decodedB.value.version &&
    stableStringify(decodedA.value.data) === stableStringify(decodedB.value.data)
  );
}

/**
 * True when the app's current state differs from what the saved view holds.
 * A failed capture counts as unmodified so we never show a misleading marker.
 */
export function isSavedViewModified(view: SavedViewV1, current: CaptureSavedViewResult): boolean {
  if (!current.ok) {
    return false;
  }
  return (
    current.tab !== view.tab ||
    !samePayload(current.payload, view.payload) ||
    stableStringify(current.extras) !== stableStringify(view.extras)
  );
}
