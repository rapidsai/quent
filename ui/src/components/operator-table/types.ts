// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from '@/services/query-plan/types';

export interface OperatorTableRow {
  partitionId: string;
  partitionLabel: string;
  scopeId: string;
  scopeLabel: string;
  parentScopeLabel: string;
  parentItemType: string;
  parentItemName: string;
  itemType: string;
  itemName: string;
  itemId: string;
  stats: Record<string, StatValue>;
}
