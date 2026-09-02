// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { FiniteStateMachine, SortDir } from '@quent/utils';

export interface EntityFilters {
  entityType: string | null;
  resourceId: string | null;
  minUsageS: string;
  windowStart: string;
  windowEnd: string;
  sortDir: SortDir;
  pageSize: number | null;
}

export interface ManualOperatorOverride {
  dagOperatorId: string | null;
  value: string | null;
}

export interface EntitiesTableState {
  filters: EntityFilters | null;
  manualOperatorOverride: ManualOperatorOverride | null;
  page: number;
  selected: FiniteStateMachine | null;
  selectedEntityId: string | null;
}

export const entitiesTableStateAtom = atom<EntitiesTableState>({
  filters: null,
  manualOperatorOverride: null,
  page: 0,
  selected: null,
  selectedEntityId: null,
});
