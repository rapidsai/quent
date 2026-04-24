// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import { atomFamily } from 'jotai-family';
import type { SortingState } from '@tanstack/react-table';
import type { AggMode } from '@/components/pivot-table/types';

/**
 * Per-pivot-table controls state, keyed by a stable `persistKey` string.
 * Atoms live at the per-query Jotai provider scope so they persist across
 * tab switches and reset on `queryId` change. A `null` value means
 * "use the hook's default" — the consumer hook is responsible for that
 * fallback.
 */

export const indexOrderAtomFamily = atomFamily(() => atom<string[] | null>(null));
export const enabledIndicesAtomFamily = atomFamily(() =>
  atom<Record<string, boolean> | null>(null)
);
export const selectedStatsAtomFamily = atomFamily(() => atom<Set<string> | null>(null));
export const statOrderAtomFamily = atomFamily(() => atom<string[] | null>(null));
export const aggModeAtomFamily = atomFamily(() => atom<AggMode | null>(null));
export const appliedDefaultKeyAtomFamily = atomFamily(() => atom<string | null>(null));
export const sortingAtomFamily = atomFamily(() => atom<SortingState | null>(null));
