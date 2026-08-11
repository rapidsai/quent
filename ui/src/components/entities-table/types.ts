// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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

export interface EntityTableRow {
  fsm: FiniteStateMachine;
  start: number;
  end: number;
  usageDurationS: number;
}
