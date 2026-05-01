// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useCallback } from 'react';
import {
  QueryToolbar,
  getOperatorColor,
  PivotedStatTable,
  PivotTableToolbar,
  getSchemaStatNames,
} from '@quent/components';
import type {
  PivotedRow,
  PivotedStatTableSchema,
  GroupedDataTableGroupKeyEntry,
} from '@quent/components';
import {
  useSelectedPlanId,
  useSelectedNodeIds,
  useHighlightedNodeIds,
  useHoveredStat,
  useStatGroupTableControls,
} from '@quent/hooks';
import type { QueryBundle, EntityRef } from '@quent/utils';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import type { OperatorTableRow } from './types';
import { buildOperatorRows, buildItemIdIndex } from './utils';

type IndexKey = 'partition' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

const OPERATOR_SCHEMA: PivotedStatTableSchema<OperatorTableRow> = {
  groups: {
    partition: {
      id: row => row.partitionId,
      label: row => row.partitionLabel,
    },
    parent_item_type: {
      id: row => row.parentItemType,
    },
    parent_item: {
      id: row => row.parentItemName,
    },
    item_type: {
      id: row => row.itemType,
    },
    item: {
      id: row => row.itemId,
      label: row => row.itemName,
    },
  },
  itemId: row => row.itemId,
  scopeId: row => row.scopeId,
  itemType: row => row.itemType,
  stats: row => row.stats,
};

const INDEX_ORDER: IndexKey[] = [
  'partition',
  'parent_item_type',
  'parent_item',
  'item_type',
  'item',
];

const DEFAULT_ENABLED: Record<IndexKey, boolean> = {
  partition: true,
  parent_item_type: false,
  parent_item: false,
  item_type: true,
  item: true,
};

// Module-scoped so the reference is stable across renders. An inline arrow
// here would be a fresh function on every render of OperatorTable, which
// cascades into PivotedStatTable's renderer dep arrays and ultimately causes
// every cell to unmount/remount on every hover atom update — see the
// `useStableRenderer` doc comment in PivotedStatTable.
const getOperatorGroupTypeColor = (key: string, id: string): string | undefined =>
  key === 'item_type' || key === 'parent_item_type'
    ? getOperatorColor(id?.toLowerCase() ?? '')
    : undefined;

// Same reasoning: an inline `{ enabled: true, overscan: 12 }` would be a
// fresh object reference per render and re-trigger virtualizer effects.
const VIRTUALIZATION_CONFIG = { enabled: true, overscan: 12 } as const;

interface OperatorTableProps {
  queryBundle: QueryBundle<EntityRef>;
}

export function OperatorTable({ queryBundle }: OperatorTableProps) {
  const selectedPlanId = useSelectedPlanId();
  const selectedNodeIds = useSelectedNodeIds();
  const [highlightState, setHighlightState] = useHighlightedNodeIds();
  const [hoveredStat, setHoveredStat] = useHoveredStat();
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const { entities } = queryBundle;
  const dagHoveredOperatorId =
    highlightState.source === 'dag' ? highlightState.primaryOperatorId : null;

  // Plans included in the table: the selected plan plus every descendant plan
  // (children, grandchildren, ...). Selecting a leaf plan yields a singleton.
  const includedPlanIds = useMemo(() => {
    if (!selectedPlanId || !entities.plans[selectedPlanId]) return new Set<string>();
    const childrenByParent = new Map<string | null, string[]>();
    for (const p of Object.values(entities.plans)) {
      if (!p) continue;
      const list = childrenByParent.get(p.parent);
      if (list) list.push(p.id);
      else childrenByParent.set(p.parent, [p.id]);
    }
    const result = new Set<string>();
    const stack: string[] = [selectedPlanId];
    while (stack.length > 0) {
      const id = stack.pop()!;
      if (result.has(id)) continue;
      result.add(id);
      const children = childrenByParent.get(id);
      if (children) stack.push(...children);
    }
    return result;
  }, [entities.plans, selectedPlanId]);

  const allRows = useMemo(
    () => buildOperatorRows(entities, includedPlanIds),
    [entities, includedPlanIds]
  );

  // When the DAG has a selection, narrow the table to just the matching
  // operator rows. If the selection is non-empty but matches nothing in the
  // current sibling-plan scope (e.g. a stage node was selected), fall back to
  // the unfiltered rows so the table doesn't appear inexplicably empty.
  const rows = useMemo(() => {
    if (selectedNodeIds.size === 0) return allRows;
    const filtered = allRows.filter(r => selectedNodeIds.has(r.itemId));
    return filtered.length > 0 ? filtered : allRows;
  }, [allRows, selectedNodeIds]);

  // Per-group-key lookup of `gk.id -> Set<itemId>`. Used by the group-cell
  // hover handlers to highlight every operator that belongs to the group.
  // The 'item' group is omitted: its highlight derives directly from
  // `row.itemIds` (single-item rows) and is handled separately.
  const itemIdsByGroupKey = useMemo(() => {
    const map = new Map<string, Map<string, Set<string>>>();
    map.set('parent_item_type', buildItemIdIndex(rows, 'parentItemType'));
    map.set('parent_item', buildItemIdIndex(rows, 'parentItemName'));
    map.set('item_type', buildItemIdIndex(rows, 'itemType', false));
    return map;
  }, [rows]);

  const allStatNames = useMemo(() => getSchemaStatNames(rows, OPERATOR_SCHEMA), [rows]);
  const hasParentItems = useMemo(() => rows.some(r => r.parentItemType !== '-'), [rows]);
  const filterIndexOrder = useCallback(
    (order: IndexKey[]) =>
      hasParentItems ? order : order.filter(k => k !== 'parent_item_type' && k !== 'parent_item'),
    [hasParentItems]
  );
  const {
    aggMode,
    setAggMode,
    selectedStats,
    orderedStatNames,
    visibleStats,
    visibleIndexOrder,
    activeIndexKeys,
    isAggregating,
    enabledIndices,
    handleToggleIndex,
    handleReorderIndex,
    handleToggleStat,
    handleSelectAllStats,
    handleSelectNoStats,
    sorting,
    setSorting,
  } = useStatGroupTableControls<IndexKey, OperatorTableRow>({
    baseIndexOrder: INDEX_ORDER,
    defaultEnabled: DEFAULT_ENABLED,
    allStatNames,
    defaultStatSelector: stats => {
      const duration = stats.filter(stat => stat === 'duration_s');
      const inputs = stats.filter(stat => stat.startsWith('input_'));
      const outputs = stats.filter(stat => stat.startsWith('output_'));
      return [...duration, ...inputs, ...outputs];
    },
    filterIndexOrder,
    persistKey: 'operatorTable',
    rows,
    getRowIndexId: (row, key) => OPERATOR_SCHEMA.groups[key].id(row),
  });

  const parentScopeLabelValue = useMemo(() => {
    for (const row of rows) {
      if (row.parentScopeLabel !== '-') return row.parentScopeLabel;
    }
    return 'Parent';
  }, [rows]);

  const scopeLabelValue = useMemo(() => {
    for (const row of rows) {
      if (row.scopeLabel !== '-' && row.scopeLabel !== parentScopeLabelValue) return row.scopeLabel;
    }
    return 'Current';
  }, [rows, parentScopeLabelValue]);

  /* This should in the future be extended with all categorical/boolean type stats */
  const indexLabels: Record<IndexKey, React.ReactNode> = useMemo(
    () => ({
      partition: 'Worker / Plan',
      parent_item_type: (
        <div>
          <div className="font-mono text-data">{parentScopeLabelValue}</div>
          <div>Operator Type</div>
        </div>
      ),
      parent_item: (
        <div>
          <div className="font-mono text-data">{parentScopeLabelValue}</div>
          <div>Operator Instance</div>
        </div>
      ),
      item_type: (
        <div>
          <div className="font-mono text-data">{scopeLabelValue}</div>
          <div>Operator Type</div>
        </div>
      ),
      item: (
        <div>
          <div className="font-mono text-data">{scopeLabelValue}</div>
          <div>Operator Instance</div>
        </div>
      ),
    }),
    [parentScopeLabelValue, scopeLabelValue]
  );

  const indexConfig = useMemo(
    () =>
      visibleIndexOrder.map(key => ({
        key,
        label: indexLabels[key],
        enabled: enabledIndices[key],
      })),
    [visibleIndexOrder, enabledIndices, indexLabels]
  );

  const getGroupCellHandlers = useCallback(
    (gk: GroupedDataTableGroupKeyEntry, row: PivotedRow) => {
      const firstItemId = row.itemIds.size === 1 ? [...row.itemIds][0] : null;

      const makeGroupHoverHandlers = (
        ids: Set<string> | null,
        primaryOperatorId: string | null = null
      ) => ({
        onMouseEnter: () =>
          setHighlightState(prev => ({ ...prev, ids, source: 'table', primaryOperatorId })),
        onMouseLeave: () =>
          setHighlightState(prev =>
            prev.source === 'table' && prev.primaryOperatorId === primaryOperatorId
              ? { ...prev, ids: null, source: null, primaryOperatorId: null }
              : prev
          ),
      });

      if (gk.key === 'item' && firstItemId) {
        return makeGroupHoverHandlers(new Set([firstItemId]), firstItemId);
      }
      const groupItems = itemIdsByGroupKey.get(gk.key);
      if (groupItems) {
        return makeGroupHoverHandlers(groupItems.get(gk.id) ?? null);
      }
      return {};
    },
    [setHighlightState, itemIdsByGroupKey]
  );

  const handleTableMouseLeave = useCallback(() => {
    setHoveredStat(null);
    setHighlightState(prev =>
      prev.source === 'table' ? { ...prev, ids: null, source: null, primaryOperatorId: null } : prev
    );
  }, [setHoveredStat, setHighlightState]);

  if (!selectedPlanId) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        Select a plan on the left to view operators
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        No operators in the selected plan
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <QueryToolbar />
      <div className="shrink-0 flex flex-col border-b border-border bg-card">
        <PivotTableToolbar
          indexConfig={indexConfig}
          isAggregating={isAggregating}
          aggMode={aggMode}
          orderedStats={orderedStatNames}
          selectedStats={selectedStats}
          onToggleIndex={handleToggleIndex}
          onReorderIndex={handleReorderIndex}
          onSetAggMode={setAggMode}
          onToggleStat={handleToggleStat}
          onSelectAllStats={handleSelectAllStats}
          onSelectNoStats={handleSelectNoStats}
        />
      </div>
      <div className="flex-1 min-h-0">
        <PivotedStatTable
          rows={rows}
          schema={OPERATOR_SCHEMA}
          activeIndices={activeIndexKeys}
          visibleStats={visibleStats}
          isAggregating={isAggregating}
          aggMode={aggMode}
          indexLabels={indexLabels}
          isDark={isDark}
          selectedItemIds={selectedNodeIds}
          hoveredItemId={dagHoveredOperatorId}
          hoveredStat={hoveredStat}
          onHoverStat={setHoveredStat}
          onTableMouseLeave={handleTableMouseLeave}
          virtualization={VIRTUALIZATION_CONFIG}
          getGroupTypeColor={getOperatorGroupTypeColor}
          getGroupCellHandlers={getGroupCellHandlers}
          sorting={sorting}
          onSortingChange={setSorting}
        />
      </div>
    </div>
  );
}
