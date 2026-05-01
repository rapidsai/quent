// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useEffect, useState } from 'react';
import type { PanelProps } from '@grafana/data';
import { useTheme2 } from '@grafana/ui';
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  setApiBaseUrl,
  DEFAULT_STALE_TIME,
  useQueryBundle,
} from '@quent/client';
import {
  PivotedStatTable,
  PivotTableToolbar,
  getOperatorColor,
  getSchemaStatNames,
} from '@quent/components';
import type {
  PivotedStatTableSchema,
  GroupedDataTableGroupKeyEntry,
} from '@quent/components';
import { useStatGroupTableControls } from '@quent/hooks';
import { buildOperatorRows, type OperatorRow } from './buildOperatorRows';
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

const INDEX_LABELS: Record<IndexKey, React.ReactNode> = {
  partition: 'Worker / Plan',
  item_type: 'Operator Type',
  item: 'Operator',
};

/**
 * Inner panel: assumes providers and `setApiBaseUrl` have already been wired.
 * Split out so the provider boilerplate does not pollute the data flow.
 */
function PivotTableBody({
  engineId,
  queryId,
  isDark,
}: {
  engineId: string;
  queryId: string;
  isDark: boolean;
}) {
  const { data: queryBundle, isLoading, error } = useQueryBundle({ engineId, queryId });

  const rows = useMemo(
    () => (queryBundle ? buildOperatorRows(queryBundle.entities) : []),
    [queryBundle]
  );
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
      const duration = stats.filter(s => s === 'duration_s');
      const inputs = stats.filter(s => s.startsWith('input_'));
      const outputs = stats.filter(s => s.startsWith('output_'));
      const picked = [...duration, ...inputs, ...outputs];
      return picked.length > 0 ? picked : null;
    },
    persistKey: `quent-pivot-${engineId}-${queryId}`,
    rows,
    getRowIndexId: (row, key) => SCHEMA.groups[key].id(row),
  });

  const indexConfig = useMemo(
    () =>
      visibleIndexOrder.map(key => ({
        key,
        label: INDEX_LABELS[key],
        enabled: enabledIndices[key],
      })),
    [visibleIndexOrder, enabledIndices]
  );

  // Hover handlers omitted: this example panel is standalone (no DAG to
  // cross-highlight into). The pivot table still works without them.
  const getGroupCellHandlers = (
    _gk: GroupedDataTableGroupKeyEntry,
    _row: unknown
  ): { onMouseEnter?: () => void; onMouseLeave?: () => void } => ({});

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        Loading query bundle…
      </div>
    );
  }
  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-destructive text-sm p-4 text-center">
        Failed to load query bundle: {error.message}
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        No operators in this query.
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
          indexLabels={INDEX_LABELS}
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
 */
export function QuentPivotTablePanel({
  options,
  width,
  height,
}: PanelProps<QuentPivotTablePanelOptions>) {
  const theme = useTheme2();
  const isDark =
    options.themeMode === 'dark'
      ? true
      : options.themeMode === 'light'
        ? false
        : theme.isDark;

  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { staleTime: DEFAULT_STALE_TIME, refetchOnWindowFocus: false },
        },
      })
  );

  // `setApiBaseUrl` is a module-global setter in `@quent/client`. Calling it
  // in an effect rather than during render keeps render pure and prevents
  // tearing if multiple panels with different apiBaseUrls coexist (last
  // mount wins; in practice all panels in a dashboard should point at the
  // same Quent instance).
  useEffect(() => {
    setApiBaseUrl(options.apiBaseUrl ?? '/api');
  }, [options.apiBaseUrl]);

  const ready = options.engineId.length > 0 && options.queryId.length > 0;

  return (
    <JotaiProvider>
      <QueryClientProvider client={queryClient}>
        <div
          style={{ width, height }}
          className={isDark ? 'dark bg-background text-foreground' : 'bg-background text-foreground'}
        >
          {ready ? (
            <PivotTableBody
              engineId={options.engineId}
              queryId={options.queryId}
              isDark={isDark}
            />
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
              Configure the panel: set <code className="px-1">Engine ID</code> and{' '}
              <code className="px-1">Query ID</code> in the panel options.
            </div>
          )}
        </div>
      </QueryClientProvider>
    </JotaiProvider>
  );
}
