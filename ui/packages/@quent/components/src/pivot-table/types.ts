// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue, ContinuousPaletteName } from '@quent/utils';
import type { AggMode, HoveredStatInfo } from '@quent/hooks';

// Re-exports of pivot-table-related types that originate in @quent/hooks but
// are commonly imported alongside the table-specific types defined below.
export type { AggMode, HoveredStatInfo } from '@quent/hooks';

/**
 * Minimal row contract for GroupedDataTable: group columns (with rowSpan) + rowKey for identity.
 */
export interface GroupedDataTableRowBase {
  groupKeys: Array<{ key: string; id: string; label: string }>;
  rowKey: string;
}

export interface GroupedDataTableSortInfo {
  desc: boolean;
}

export interface GroupedDataTableGroupKeyEntry {
  key: string;
  id: string;
  label: string;
}

// --- GroupedDataTable renderer component prop types ---

export interface DataHeaderProps {
  stat: string;
  sortInfo: GroupedDataTableSortInfo | null;
  onSort: () => void;
  className?: string;
  style?: React.CSSProperties;
}

export interface GroupCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  groupKey: GroupedDataTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  style?: React.CSSProperties;
  className?: string;
}

export interface DataCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  stat: string;
}

export interface PivotTableGroupCellHoverHandlers {
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}

export interface PivotTableInteractionConfig<TRow extends GroupedDataTableRowBase> {
  hoveredStat: HoveredStatInfo | null;
  setHoveredStat: (info: HoveredStatInfo | null) => void;
  hoveredItemId?: string | null;
  selectedItemIds?: Set<string>;
  onTableMouseLeave?: () => void;
  groupCellHandlers?: (
    groupKey: GroupedDataTableGroupKeyEntry,
    row: TRow
  ) => PivotTableGroupCellHoverHandlers;
}

export interface PivotTableRenderConfig {
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
}

export interface PivotTableDnDConfig {
  draggedStat: string | null;
  getDropTargetPosition?: (statName: string) => 'before' | 'after' | undefined;
  onStatDragStart: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragOver: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragLeave: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDrop: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragEnd: () => void;
}

export interface PivotTableDisplayConfig {
  isAggregating: boolean;
  aggMode: AggMode;
  colorPalette: ContinuousPaletteName;
  darkMode: boolean;
}

// --- PivotedStatTable types ---

export type SortDir = 'asc' | 'desc';

export interface StatGroupInputGroupValue {
  id: string;
  label: string;
}

export interface StatGroupExpandedRow {
  groups: Record<string, StatGroupInputGroupValue>;
  itemType: string;
  itemId: string;
  scopeId: string;
  statisticName: string;
  value: StatValue;
}

export interface PivotedStatTableSchema<TRow> {
  /**
   * Group dimensions keyed by group id (e.g. partition, item_type, item).
   * These keys are referenced by activeIndices and indexLabels.
   */
  groups: Record<
    string,
    {
      id: (row: TRow) => string;
      label?: (row: TRow) => string;
    }
  >;
  /** Unique item identity used for hover/selection linkage. */
  itemId: (row: TRow) => string;
  /** Scope identity used for cross-view selection linkage. */
  scopeId: (row: TRow) => string;
  /**
   * Optional item type fallback. If omitted, uses group "item_type" id, then "item" id.
   */
  itemType?: (row: TRow) => string;
  /** Stat map for one logical row; keys become table stat columns. */
  stats: (row: TRow) => Record<string, StatValue>;
}

export interface GroupKeyEntry {
  key: string;
  id: string;
  label: string;
}

export interface PivotedRowAgg {
  sum: number | bigint | null;
  mean: number | null;
  min: number | bigint | null;
  max: number | bigint | null;
  stdev: number | null;
  count: number;
  isNumeric: boolean;
}

export interface PivotedRow {
  groupKeys: GroupKeyEntry[];
  rowKey: string;
  values: Map<string, StatValue>;
  aggs: Map<string, PivotedRowAgg>;
  itemIds: Set<string>;
  itemType: string;
  /** Map from item ID to the scope ID it belongs to */
  itemScopeIds: Map<string, string>;
}
