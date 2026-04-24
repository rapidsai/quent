// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';

/**
 * Per-query resource-tree state. Lives at the per-query Jotai provider scope
 * (keyed by `queryId` in `routes/profile.engine.$engineId.tsx`) so it
 * persists across `/timeline` ↔ `/operators` tab switches and resets on
 * query change.
 */

export const expandedIdsAtom = atom<Set<string>>(new Set<string>());
export const selectedTypesAtom = atom<Map<string, string>>(new Map<string, string>());
export const selectedFsmTypesAtom = atom<Map<string, string | null>>(
  new Map<string, string | null>()
);
export const rootResourceTypeAtom = atom<string | null>(null);
