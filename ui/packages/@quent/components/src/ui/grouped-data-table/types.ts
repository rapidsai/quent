// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { CSSProperties } from 'react';

export interface GroupedDataTableGroupKeyEntry {
  key: string;
  id: string;
  label: string;
}

export interface GroupedDataTableRowBase {
  groupKeys: GroupedDataTableGroupKeyEntry[];
  rowKey: string;
}

export interface GroupedDataTableSortInfo {
  desc: boolean;
}

export interface DataHeaderProps {
  stat: string;
  sortInfo: GroupedDataTableSortInfo | null;
  onSort: () => void;
  className?: string;
  style?: CSSProperties;
}

export interface GroupCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  groupKey: GroupedDataTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  style?: CSSProperties;
  className?: string;
}

export interface DataCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  stat: string;
}
