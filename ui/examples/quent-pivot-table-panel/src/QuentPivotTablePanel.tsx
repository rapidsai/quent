// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo, useState } from 'react';
import type { PanelProps } from '@grafana/data';
import { useTheme2 } from '@grafana/ui';
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { StatValue } from '@quent/utils';
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

/**
 * The base hierarchy is fixed (`partition` → `item_type` → `item`); the
 * panel can splice additional categorical group-bys between `item_type`
 * and `item` via the `groupByColumns` option. Extra keys are namespaced
 * with `extra:` so they cannot collide with the base keys, even if a
 * dataset has a column literally named `partition`.
 */
const EXTRA_GROUP_PREFIX = 'extra:';

// Module-scoped to keep the reference stable across renders. Inline arrows
// here cascade into PivotedStatTable's renderer dep arrays and force every
// cell to remount on every hover atom update — see the `useStableRenderer`
// doc comment in PivotedStatTable.
const getGroupTypeColor = (key: string, id: string): string | undefined =>
  key === 'item_type' ? getOperatorColor(id?.toLowerCase() ?? '') : undefined;

// Virtualization is off in the example: the demo CSVs are small (≤40 rows)
// and the virtualizer's compact group renderer collapses the row-span
// grouping that makes the unaggregated view readable. Flip `enabled: true`
// (and adjust `overscan`) when wiring this panel up to a large datasource.
const VIRTUALIZATION_CONFIG = { enabled: false } as const;

function parseGroupByColumns(raw: string): string[] {
  return raw
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
}

/**
 * Resolve user-supplied column names against the actual stat keys present
 * in the rows, case-insensitively. Returns the canonical (as-it-appears
 * in the data) names so downstream lookups don't have to lowercase on
 * every cell render.
 */
function resolveExtraGroupColumns(rows: OperatorRow[], requested: string[]): string[] {
  if (requested.length === 0) return [];
  const canonical = new Map<string, string>();
  for (const r of rows) {
    for (const k of Object.keys(r.stats)) {
      const lower = k.toLowerCase();
      if (!canonical.has(lower)) canonical.set(lower, k);
    }
  }
  const out: string[] = [];
  const seen = new Set<string>();
  for (const name of requested) {
    const found = canonical.get(name.toLowerCase());
    if (found && !seen.has(found)) {
      out.push(found);
      seen.add(found);
    }
  }
  return out;
}

function statValueToString(v: StatValue | undefined): string {
  if (v == null || v === '') return '-';
  return String(v);
}

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
  extraGroupColumns,
}: {
  rows: OperatorRow[];
  isDark: boolean;
  persistKey: string;
  indexLabels: Record<string, React.ReactNode>;
  extraGroupColumns: string[];
}) {
  const schema = useMemo<PivotedStatTableSchema<OperatorRow>>(() => {
    const extraGroups: PivotedStatTableSchema<OperatorRow>['groups'] = {};
    for (const col of extraGroupColumns) {
      // Each extra column becomes its own group keyed by `extra:<col>`.
      // The id closure captures `col` (stable for this schema instance);
      // string-coerce the value so non-string stats (e.g. numeric Doors)
      // produce a stable group key.
      extraGroups[`${EXTRA_GROUP_PREFIX}${col}`] = {
        id: r => statValueToString(r.stats[col]),
      };
    }

    const excluded = new Set(extraGroupColumns);
    return {
      groups: {
        partition: { id: r => r.partitionId, label: r => r.partitionLabel },
        item_type: { id: r => r.itemType },
        ...extraGroups,
        item: { id: r => r.itemId, label: r => r.itemName },
      },
      itemId: r => r.itemId,
      scopeId: r => r.scopeId,
      itemType: r => r.itemType,
      // Strip promoted columns out of stats so they don't show up as
      // (mostly meaningless) numeric aggregates in the column picker.
      stats:
        excluded.size === 0
          ? r => r.stats
          : r => {
              const out: Record<string, StatValue> = {};
              for (const [k, v] of Object.entries(r.stats)) {
                if (!excluded.has(k)) out[k] = v;
              }
              return out;
            },
    };
  }, [extraGroupColumns]);

  const baseIndexOrder = useMemo<string[]>(
    () => [
      'partition',
      'item_type',
      ...extraGroupColumns.map(c => `${EXTRA_GROUP_PREFIX}${c}`),
      'item',
    ],
    [extraGroupColumns]
  );

  const defaultEnabled = useMemo<Record<string, boolean>>(
    () => Object.fromEntries(baseIndexOrder.map(k => [k, true])),
    [baseIndexOrder]
  );

  // Drop any persisted index keys that no longer exist in the current
  // schema (e.g. user removed a column from `groupByColumns`) and inject
  // freshly-added keys at the end so the toolbar always reflects the
  // configured grouping shape.
  const filterIndexOrder = useCallback(
    (order: string[]): string[] => {
      const valid = new Set(baseIndexOrder);
      const filtered = order.filter(k => valid.has(k));
      for (const k of baseIndexOrder) {
        if (!filtered.includes(k)) filtered.push(k);
      }
      return filtered;
    },
    [baseIndexOrder]
  );

  const allStatNames = useMemo(() => getSchemaStatNames(rows, schema), [rows, schema]);

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
  } = useStatGroupTableControls<string, OperatorRow>({
    baseIndexOrder,
    defaultEnabled,
    allStatNames,
    // Default to a small, readable subset; user can add more from the toolbar.
    defaultStatSelector: stats => {
      const duration = stats.filter(s => /duration/i.test(s));
      const inputs = stats.filter(s => /^input[_-]/i.test(s));
      const outputs = stats.filter(s => /^output[_-]/i.test(s));
      const picked = [...duration, ...inputs, ...outputs];
      return picked.length > 0 ? picked : null;
    },
    filterIndexOrder,
    persistKey,
    rows,
    getRowIndexId: (row, key) => schema.groups[key]?.id(row) ?? '',
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
          schema={schema}
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

  // Resolve the user-supplied column list against the data's actual stat
  // keys so casing differences between the option text and the datasource
  // don't silently drop columns. Stabilize the array identity by content
  // key — `rows` is rebuilt on every Grafana data tick, but as long as
  // the resolved column set is unchanged we want downstream memos to
  // skip recomputing the schema / index order.
  const extraGroupColumnsRaw = useMemo(
    () => resolveExtraGroupColumns(rows, parseGroupByColumns(options.groupByColumns)),
    [rows, options.groupByColumns]
  );
  const extraGroupColumnsKey = extraGroupColumnsRaw.join('\0');
  const extraGroupColumns = useMemo(
    () => extraGroupColumnsRaw,
    // Intentionally only depend on the content key; ignoring the array
    // identity is the whole point of this stabilizer.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [extraGroupColumnsKey]
  );

  const indexLabels = useMemo<Record<string, React.ReactNode>>(() => {
    const labels: Record<string, React.ReactNode> = {
      partition: options.partitionLabel,
      item_type: options.itemTypeLabel,
      item: options.itemLabel,
    };
    for (const col of extraGroupColumns) {
      labels[`${EXTRA_GROUP_PREFIX}${col}`] = col;
    }
    return labels;
  }, [options.partitionLabel, options.itemTypeLabel, options.itemLabel, extraGroupColumns]);

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
            extraGroupColumns={extraGroupColumns}
          />
        </div>
      </QueryClientProvider>
    </JotaiProvider>
  );
}
