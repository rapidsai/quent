// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from '@quent/utils';

export type OperatorTableIndexKey =
  'partition' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

export const OPERATOR_TABLE_PERSIST_KEY = 'operatorTable';
export const OPERATOR_TABLE_INDEX_ORDER: [OperatorTableIndexKey, ...OperatorTableIndexKey[]] = [
  'partition',
  'parent_item_type',
  'parent_item',
  'item_type',
  'item',
];
export const DEFAULT_OPERATOR_TABLE_ENABLED: Record<OperatorTableIndexKey, boolean> = {
  partition: true,
  parent_item_type: false,
  parent_item: false,
  item_type: true,
  item: true,
};

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
  /** Maps stat key → quantity name (key into QueryBundle.quantity_specs) for stats that have one. */
  statQuantities: Record<string, string>;
}
