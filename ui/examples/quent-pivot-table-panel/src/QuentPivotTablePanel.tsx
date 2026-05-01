// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import type { PanelProps } from '@grafana/data';
import { useTheme2 } from '@grafana/ui';
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  PivotedStatTable,
  PivotTableToolbar,
  getOperatorColor,
  getSchemaStatNames,
} from '@quent/components';
import type { PivotedStatTableSchema, GroupedDataTableGroupKeyEntry } from '@quent/components';
import { useStatGroupTableControls } from '@quent/hooks';
import { frameToOperatorRows, type OperatorRow } from './frameToOperatorRows';
import type { QuentPivotTablePanelOptions } from './types';

type IndexKey = 'partition' | 'item_type' | 'item';

const SCHEMA: PivotedStatTableSchema<OperatorRow> = {
  groups: {
    partition: { id: r => r.partitionId, label: r => r.partitionLabel },
    item_type: { id: r => r.itemType },
    item: { id: r => r.itemId, label: r => r.itemName },
  },
  itemId: r => r.itemId,
  scopeId: r => r.scopeId,
  itemType: r => r.itemType,
  stats: r => r.stats,
};

const INDEX_ORDER: IndexKey[] = ['partition', 'item_type', 'item'];
const DEFAULT_ENABLED: Record<IndexKey, boolean> = {
  partition: true,
  item_type: true,
  item: true,
};

// Module-scoped to keep the reference stable across renders. Inline arrows
// here cascade into PivotedStatTable's renderer dep arrays and force every
// cell to remount on every hover atom update — see the `useStableRenderer`
// doc comment in PivotedStatTable.
const getGroupTypeColor = (key: string, id: string): string | undefined =>
  key === 'item_type' ? getOperatorColor(id?.toLowerCase() ?? '') : undefined;

const VIRTUALIZATION_CONFIG = { enabled: true, overscan: 12 } as const;

/**
 * Inner panel: assumes providers are wired and `rows` have been adapted from
 * the Grafana `PanelData`. Split out so the provider boilerplate does not
 * pollute the data flow.
 */
function PivotTableBody({
  rows,
  isDark,
  persistKey,
  indexLabels,
}: {
  rows: OperatorRow[];
  isDark: boolean;
  persistKey: string;
  indexLabels: Record<IndexKey, React.ReactNode>;
}) {
  const allStatNames = useMemo(() => getSchemaStatNames(rows, SCHEMA), [rows]);

  const {
    aggMode,
    setAggMode,
    selectedStats,
    orderedStatNames,
    visibleStats,
    activeIndexKeys,
    visibleIndexOrder,
    isAggregating,
    enabledIndices,
    handleToggleIndex,
    handleReorderIndex,
    handleToggleStat,
    handleSelectAllStats,
    handleSelectNoStats,
    sorting,
    setSorting,
  } = useStatGroupTableControls<IndexKey, OperatorRow>({
    baseIndexOrder: INDEX_ORDER,
    defaultEnabled: DEFAULT_ENABLED,
    allStatNames,
    // Default to a small, readable subset; user can add more from the toolbar.
    defaultStatSelector: stats => {
      const duration = stats.filter(s => /duration/i.test(s));
      const inputs = stats.filter(s => /^input[_-]/i.test(s));
      const outputs = stats.filter(s => /^output[_-]/i.test(s));
      const picked = [...duration, ...inputs, ...outputs];
      return picked.length > 0 ? picked : null;
    },
    persistKey,
    rows,
    getRowIndexId: (row, key) => SCHEMA.groups[key].id(row),
  });

  const indexConfig = useMemo(
    () =>
      visibleIndexOrder.map(key => ({
        key,
        label: indexLabels[key],
        enabled: enabledIndices[key],
      })),
    [visibleIndexOrder, enabledIndices, indexLabels]
  );

  // Hover handlers omitted: this example panel is standalone (no DAG to
  // cross-highlight into). The pivot table still works without them.
  const getGroupCellHandlers = (
    _gk: GroupedDataTableGroupKeyEntry,
    _row: unknown
  ): { onMouseEnter?: () => void; onMouseLeave?: () => void } => ({});

  if (rows.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
        No rows in the panel data. Configure a query that returns at least an{' '}
        <code className="px-1">item_id</code> column.
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
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
          schema={SCHEMA}
          activeIndices={activeIndexKeys}
          visibleStats={visibleStats}
          isAggregating={isAggregating}
          aggMode={aggMode}
          indexLabels={indexLabels}
          isDark={isDark}
          virtualization={VIRTUALIZATION_CONFIG}
          getGroupTypeColor={getGroupTypeColor}
          getGroupCellHandlers={getGroupCellHandlers}
          sorting={sorting}
          onSortingChange={setSorting}
        />
      </div>
    </div>
  );
}

/**
 * Top-level Grafana panel component. One QueryClient + Jotai store *per
 * panel instance* so dashboards with multiple Quent panels don't share
 * sort/visibility state.
 *
 * Data plane: the panel reads `props.data.series` (Grafana `DataFrame[]`)
 * and adapts it to `OperatorRow[]`. Any datasource that produces the
 * documented columns can drive it — see `frameToOperatorRows.ts` and
 * `provisioning/dashboards/quent-pivot-demo.json` for the expected shape.
 */
export function QuentPivotTablePanel({
  id,
  data,
  options,
  width,
  height,
}: PanelProps<QuentPivotTablePanelOptions>) {
  const theme = useTheme2();
  const isDark =
    options.themeMode === 'dark' ? true : options.themeMode === 'light' ? false : theme.isDark;

  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { staleTime: 60_000, refetchOnWindowFocus: false },
        },
      })
  );

  const rows = useMemo(() => frameToOperatorRows(data?.series), [data?.series]);

  const indexLabels = useMemo<Record<IndexKey, React.ReactNode>>(
    () => ({
      partition: options.partitionLabel,
      item_type: options.itemTypeLabel,
      item: options.itemLabel,
    }),
    [options.partitionLabel, options.itemTypeLabel, options.itemLabel]
  );

  // Per-panel persistence key: scopes toolbar/sort state to this panel
  // instance so two pivot panels in one dashboard do not clobber each other.
  const persistKey = `quent-pivot-panel-${id}`;

  return (
    <JotaiProvider>
      <QueryClientProvider client={queryClient}>
        <div
          style={{ width, height }}
          className={
            isDark ? 'dark bg-background text-foreground' : 'bg-background text-foreground'
          }
        >
          <PivotTableBody
            rows={rows}
            isDark={isDark}
            persistKey={persistKey}
            indexLabels={indexLabels}
          />
        </div>
      </QueryClientProvider>
    </JotaiProvider>
  );
}
