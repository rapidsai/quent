// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';

/** Set of expanded item IDs in the resource tree */
export const expandedIdsAtom = atom<Set<string>>(new Set<string>());

/** Per-item selected resource type in the resource tree */
export const selectedTypesAtom = atom<Map<string, string>>(new Map<string, string>());

/** Per-item selected FSM filter type in the resource tree */
export const selectedFsmTypesAtom = atom<Map<string, string | null>>(
  new Map<string, string | null>()
);
