// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { FiniteStateMachine } from '@quent/utils';

export type { EntityFilters } from '@/atoms/entitiesTable';

export interface EntityTableRow {
  fsm: FiniteStateMachine;
  start: number;
  end: number;
  usageDurationS: number;
}
