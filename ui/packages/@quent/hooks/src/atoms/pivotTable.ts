// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// PRIVATE to @quent/hooks — consumers use the selector hook
// `useStatGroupTableControls` exported from the package index.

import { atom } from 'jotai';
import { atomFamily } from 'jotai-family';
import type { SortingState } from '@tanstack/react-table';

/**
 * Aggregation mode for pivot-table cells. Defined here (rather than in
 * `@quent/components`) so the atoms below can type their stored values
 * without creating a circular dependency on the components package.
 */
export type AggMode = 'value' | 'sum' | 'mean' | 'min' | 'max' | 'stdev';

/**
 * Per-pivot-table controls state, keyed by a stable `persistKey` string.
 * Atoms live at the surrounding Jotai provider scope so they persist across
 * tab switches and reset when the provider remounts. A `null` value means
 * "use the consumer hook's default" — the consumer hook is responsible for
 * that fallback.
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
