// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import type { ColumnDef } from '@tanstack/react-table';
import { useAtomValue } from 'jotai';
import { PivotTable } from './PivotTable';
import { cn } from '@/lib/utils';
import type { AggMode, PivotedRow, HoveredStatInfo, StatGroupTableSchema } from './types';
import type { PivotTableGroupKeyEntry, PivotTableSortInfo } from './types';
import {
  buildPivotedRows,
  expandRowsFromSchema,
  formatStatValue,
  formatStatNumber,
  getSchemaStatNames,
  getSortValue,
  gradientBg,
} from './utils';
import type { PivotTableGroupRenderMode, PivotTableVirtualizationOptions } from './PivotTable';
import { nodeColorPaletteAtom } from '@/atoms/dag';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import type { ContinuousPaletteName } from '@/services/colors';

const HIGHLIGHT_WASH = 'inset 0 0 0 999px hsl(var(--primary) / 0.07)';

function DataHeader({
  stat,
  sortInfo,
  onSort,
  className,
  style,
  rowHeaderHoverActive,
  draggedStat,
  setDraggedStat,
  onReorderStat,
  onHoverStat,
  buildHoveredStatInfo,
  hoveredStatName,
}: {
  stat: string;
  sortInfo: PivotTableSortInfo | null;
  onSort: () => void;
  className?: string;
  style?: React.CSSProperties;
  rowHeaderHoverActive?: boolean;
  draggedStat: string | null;
  setDraggedStat: (stat: string | null) => void;
  onReorderStat?: (from: string, to: string) => void;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  buildHoveredStatInfo: (statName: string) => HoveredStatInfo | null;
  hoveredStatName: string | undefined;
}) {
  return (
    <th
      draggable
      onDragStart={() => setDraggedStat(stat)}
      onDragOver={e => {
        e.preventDefault();
        if (!draggedStat || draggedStat === stat) return;
        onReorderStat?.(draggedStat, stat);
        setDraggedStat(stat);
      }}
      onDragEnd={() => setDraggedStat(null)}
      onClick={onSort}
      onMouseEnter={() => onHoverStat?.(buildHoveredStatInfo(stat))}
      onMouseLeave={() => onHoverStat?.(null)}
      className={cn(
        'table-header-overlay text-right px-3 py-2 text-sm font-mono text-data whitespace-nowrap cursor-pointer select-none hover:text-foreground',
        className,
        {
          'font-semibold': Boolean(rowHeaderHoverActive),
          'opacity-50': draggedStat === stat,
          'text-foreground': Boolean(sortInfo),
          'table-header-overlay-active': hoveredStatName === stat || Boolean(rowHeaderHoverActive),
        }
      )}
      style={style}
    >
      <span className="relative z-10">
        {stat}
        {sortInfo && <span className="ml-1 text-xs">{sortInfo.desc ? '▼' : '▲'}</span>}
      </span>
    </th>
  );
}

function GroupCell({
  row,
  groupKey: gk,
  rowSpan,
  style,
  className,
  getGroupTypeColor,
  getGroupCellHandlers,
  hoveredHeaderItemIds,
  setHoveredHeaderItemIds,
  hoveredItemId,
}: {
  row: PivotedRow;
  groupKey: PivotTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  style?: React.CSSProperties;
  className?: string;
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  hoveredHeaderItemIds?: Set<string> | null;
  setHoveredHeaderItemIds?: (itemIds: Set<string> | null) => void;
  hoveredItemId?: string | null;
  getGroupCellHandlers?: (
    groupKey: PivotTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
}) {
  const typeColor = getGroupTypeColor?.(gk.key, gk.id);
  const handlers = getGroupCellHandlers?.(gk, row);
  const isRowHeaderHighlightedFromTable =
    hoveredHeaderItemIds != null && [...row.itemIds].some(id => hoveredHeaderItemIds.has(id));
  const isRowHighlightedFromDag =
    hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId);
  const isRowHeaderHighlighted = isRowHeaderHighlightedFromTable || isRowHighlightedFromDag;
  const baseStyle = typeColor
    ? {
        ...style,
        borderLeftWidth: 8,
        borderLeftColor: typeColor,
        // Opaque tint so scrolled rows never bleed through sticky group cells.
        backgroundColor: `color-mix(in srgb, ${typeColor} 15%, hsl(var(--card)))`,
      }
    : style;
  const mergedStyle = {
    ...baseStyle,
    boxShadow: baseStyle?.boxShadow,
  };

  return (
    <td
      className={cn(
        'table-header-overlay px-3 py-1.5 whitespace-nowrap align-top border-r border-border/30',
        isRowHeaderHighlighted && 'table-header-overlay-active',
        className
      )}
      rowSpan={rowSpan}
      style={mergedStyle}
      onMouseEnter={() => {
        setHoveredHeaderItemIds?.(new Set(row.itemIds));
        handlers?.onMouseEnter?.();
      }}
      onMouseLeave={() => {
        setHoveredHeaderItemIds?.(null);
        handlers?.onMouseLeave?.();
      }}
    >
      <span className="relative z-10">{gk.label}</span>
    </td>
  );
}

function DataCell({
  row,
  stat,
  isAggregating,
  aggMode,
  columnRanges,
  colorPalette,
  darkMode,
  hoveredHeaderItemIds,
  hoveredItemId,
  hoveredStatName,
  onHoverStat,
  buildHoveredStatInfo,
}: {
  row: PivotedRow;
  stat: string;
  isAggregating: boolean;
  aggMode: AggMode;
  columnRanges: Map<string, { min: number; max: number }>;
  colorPalette: ContinuousPaletteName;
  darkMode: boolean;
  hoveredHeaderItemIds?: Set<string> | null;
  hoveredItemId?: string | null;
  hoveredStatName: string | undefined;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  buildHoveredStatInfo: (statName: string) => HoveredStatInfo | null;
}) {
  const numVal = getSortValue(row, stat, isAggregating, aggMode);
  const range = columnRanges.get(stat);
  const bg =
    numVal !== null && range
      ? gradientBg(numVal, range.min, range.max, colorPalette, darkMode)
      : undefined;
  const isStatHovered = hoveredStatName === stat;
  const colHighlight = isStatHovered ? HIGHLIGHT_WASH : undefined;
  const isRowHighlightedFromTable =
    hoveredHeaderItemIds != null && [...row.itemIds].some(id => hoveredHeaderItemIds.has(id));
  const isRowHighlightedFromDag =
    hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId);
  const rowHighlight =
    isRowHighlightedFromTable || isRowHighlightedFromDag ? HIGHLIGHT_WASH : undefined;
  const cellHighlight = rowHighlight ?? colHighlight;
  const statCellProps = {
    onMouseEnter: () => onHoverStat?.(buildHoveredStatInfo(stat)),
    onMouseLeave: () => onHoverStat?.(null),
  };
  if (!isAggregating) {
    const val = row.values.get(stat) ?? null;
    return (
      <td
        className="relative z-0 px-3 py-1.5 whitespace-nowrap text-right font-mono"
        style={{ backgroundColor: bg, boxShadow: cellHighlight }}
        {...statCellProps}
      >
        {formatStatValue(val, stat)}
      </td>
    );
  }
  const agg = row.aggs.get(stat);
  if (!agg || !agg.isNumeric) {
    return (
      <td
        className="relative z-0 px-3 py-1.5 whitespace-nowrap text-right font-mono text-muted-foreground"
        style={{ boxShadow: cellHighlight }}
        {...statCellProps}
      >
        -
      </td>
    );
  }
  const displayVal = agg[aggMode as Exclude<AggMode, 'value'>] ?? null;
  return (
    <td
      className="relative z-0 px-3 py-1.5 whitespace-nowrap text-right font-mono"
      style={{ backgroundColor: bg, boxShadow: cellHighlight }}
      {...statCellProps}
    >
      {formatStatNumber(displayVal, stat)}
    </td>
  );
}

interface StatGroupTableProps<TRow> {
  rows: TRow[];
  schema: StatGroupTableSchema<TRow>;
  activeIndices: string[];
  visibleStats?: string[];
  isAggregating?: boolean;
  aggMode?: AggMode;
  indexLabels?: Record<string, React.ReactNode>;
  // interaction state
  selectedItemIds?: Set<string>;
  hoveredItemId?: string | null;
  hoveredStat?: HoveredStatInfo | null;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  // rendering
  virtualization?: PivotTableVirtualizationOptions;
  groupRenderMode?: PivotTableGroupRenderMode;
  stickyGroupColumns?: boolean;
  // display config
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  getGroupCellHandlers?: (
    groupKey: PivotTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
  onReorderStat?: (from: string, to: string) => void;
}

export function StatGroupTable<TRow>({
  rows,
  schema,
  activeIndices,
  visibleStats,
  isAggregating = false,
  aggMode = 'sum',
  indexLabels,
  selectedItemIds,
  hoveredItemId,
  hoveredStat,
  onHoverStat,
  virtualization,
  groupRenderMode,
  stickyGroupColumns = true,
  getGroupTypeColor,
  getGroupCellHandlers,
  onReorderStat,
}: StatGroupTableProps<TRow>) {
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  const rowRefs = useRef<Map<string, HTMLTableRowElement>>(new Map());
  const [draggedStat, setDraggedStat] = useState<string | null>(null);
  const [hoveredHeaderItemIds, setHoveredHeaderItemIds] = useState<Set<string> | null>(null);
  const [tableStatOrder, setTableStatOrder] = useState<string[]>([]);
  const [uncontrolledHoveredStat, setUncontrolledHoveredStat] = useState<HoveredStatInfo | null>(
    null
  );
  const effectiveHoveredStat = hoveredStat ?? uncontrolledHoveredStat;
  const emitHoverStat = onHoverStat ?? setUncontrolledHoveredStat;
  const expandedRows = useMemo(() => expandRowsFromSchema(rows, schema), [rows, schema]);
  const resolvedVisibleStats = useMemo(
    () => visibleStats ?? getSchemaStatNames(rows, schema),
    [rows, schema, visibleStats]
  );
  const resolvedVisibleStatsKey = useMemo(
    () => resolvedVisibleStats.join('\0'),
    [resolvedVisibleStats]
  );
  useEffect(() => {
    setTableStatOrder(prev => {
      if (prev.length === 0) return resolvedVisibleStats;
      const visibleSet = new Set(resolvedVisibleStats);
      const kept = prev.filter(stat => visibleSet.has(stat));
      const additions = resolvedVisibleStats.filter(stat => !kept.includes(stat));
      return [...kept, ...additions];
    });
  }, [resolvedVisibleStats, resolvedVisibleStatsKey]);
  const effectiveVisibleStats = tableStatOrder.length > 0 ? tableStatOrder : resolvedVisibleStats;
  const resolvedIndexLabels = useMemo(
    () =>
      indexLabels ??
      (Object.fromEntries(activeIndices.map(key => [key, key])) as Record<string, React.ReactNode>),
    [activeIndices, indexLabels]
  );
  const effectiveGroupRenderMode =
    groupRenderMode ?? (virtualization?.enabled ? 'compact' : 'rowSpan');
  const rowHeaderHoverActive = hoveredHeaderItemIds !== null;

  const statsByItem = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    for (const row of expandedRows) {
      const v = typeof row.value === 'number' ? row.value : null;
      if (v === null) continue;
      let itemMap = map.get(row.statisticName);
      if (!itemMap) {
        itemMap = new Map();
        map.set(row.statisticName, itemMap);
      }
      itemMap.set(row.itemId, v);
    }
    return map;
  }, [expandedRows]);

  const buildHoveredStatInfo = useCallback(
    (statName: string): HoveredStatInfo | null => {
      const values = statsByItem.get(statName);
      if (!values || values.size === 0) return null;
      let min = Infinity,
        max = -Infinity;
      for (const v of values.values()) {
        if (v < min) min = v;
        if (v > max) max = v;
      }
      return { name: statName, values, min, max };
    },
    [statsByItem]
  );

  const pivotedRows = useMemo(
    () =>
      buildPivotedRows(
        expandedRows,
        activeIndices.map(key => ({
          key,
          getId: row => row.groups[key]?.id ?? '-',
          getLabel: row => row.groups[key]?.label ?? row.groups[key]?.id ?? '-',
        })),
        isAggregating
      ),
    [expandedRows, activeIndices, isAggregating]
  );

  const columnRanges = useMemo(() => {
    const ranges = new Map<string, { min: number; max: number }>();
    for (const stat of effectiveVisibleStats) {
      let min = Infinity;
      let max = -Infinity;
      for (const row of pivotedRows) {
        const v = getSortValue(row, stat, isAggregating, aggMode);
        if (v !== null) {
          if (v < min) min = v;
          if (v > max) max = v;
        }
      }
      if (min !== Infinity) ranges.set(stat, { min, max });
    }
    return ranges;
  }, [pivotedRows, effectiveVisibleStats, isAggregating, aggMode]);

  const handleReorderStat = useCallback(
    (from: string, to: string) => {
      setTableStatOrder(prev => {
        const next = prev.length > 0 ? [...prev] : [...resolvedVisibleStats];
        const fromIndex = next.indexOf(from);
        const toIndex = next.indexOf(to);
        if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return next;
        const [moved] = next.splice(fromIndex, 1);
        next.splice(toIndex, 0, moved);
        return next;
      });
      // Keep optional callback for external listeners, without requiring global reordering.
      onReorderStat?.(from, to);
    },
    [onReorderStat, resolvedVisibleStats]
  );

  useEffect(() => {
    if (!hoveredItemId) return;
    const row = pivotedRows.find(r => r.itemIds.has(hoveredItemId));
    if (!row) return;
    const el = rowRefs.current.get(row.rowKey);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }, [hoveredItemId, pivotedRows]);

  const sharedProps = useMemo(
    () => ({
      draggedStat,
      setDraggedStat,
      onReorderStat: handleReorderStat,
      onHoverStat: emitHoverStat,
      buildHoveredStatInfo,
      hoveredStatName: effectiveHoveredStat?.name,
      getGroupTypeColor,
      getGroupCellHandlers,
      isAggregating,
      aggMode,
      columnRanges,
      colorPalette: nodePalette,
      darkMode: isDarkMode,
      hoveredHeaderItemIds,
      hoveredItemId,
      setHoveredHeaderItemIds,
      rowHeaderHoverActive,
    }),
    [
      draggedStat,
      handleReorderStat,
      emitHoverStat,
      buildHoveredStatInfo,
      effectiveHoveredStat?.name,
      getGroupTypeColor,
      getGroupCellHandlers,
      isAggregating,
      aggMode,
      columnRanges,
      nodePalette,
      isDarkMode,
      hoveredHeaderItemIds,
      hoveredItemId,
      rowHeaderHoverActive,
    ]
  );

  const columns = useMemo((): ColumnDef<PivotedRow>[] => {
    const groupCols: ColumnDef<PivotedRow>[] = activeIndices.map(def => ({
      id: def,
      header: String(resolvedIndexLabels[def] ?? def),
      enableSorting: false,
    }));
    const statCols: ColumnDef<PivotedRow>[] = effectiveVisibleStats.map(stat => ({
      id: stat,
      header: stat,
      enableSorting: true,
      sortUndefined: 'last',
      accessorFn: (row: PivotedRow) => getSortValue(row, stat, isAggregating, aggMode) ?? undefined,
    }));
    return [...groupCols, ...statCols];
  }, [activeIndices, effectiveVisibleStats, resolvedIndexLabels, isAggregating, aggMode]);

  const hasSelection = (selectedItemIds?.size ?? 0) > 0;
  const isSelected = (row: PivotedRow) => [...row.itemIds].some(id => selectedItemIds?.has(id));

  return (
    <PivotTable
      data={pivotedRows}
      columns={columns}
      getRowId={row => row.rowKey}
      groupColumnIds={activeIndices}
      renderGroupHeader={columnId => (
        <span className={cn('relative z-10', { 'font-semibold': rowHeaderHoverActive })}>
          {resolvedIndexLabels[columnId]}
        </span>
      )}
      sharedProps={sharedProps}
      DataHeader={DataHeader}
      GroupCell={GroupCell}
      DataCell={DataCell}
      virtualization={virtualization}
      groupRenderMode={effectiveGroupRenderMode}
      stickyGroupColumns={stickyGroupColumns}
      getRowRef={rowKey => el => {
        if (el) rowRefs.current.set(rowKey, el);
        else rowRefs.current.delete(rowKey);
      }}
      getRowClassName={row =>
        cn('border-b border-border/50 hover:bg-muted/50 transition-opacity', {
          'bg-muted/70': isSelected(row),
        })
      }
      getRowStyle={row => {
        const isHoveredFromDag =
          hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId);
        const isDimmed = hasSelection && !isSelected(row) && !isHoveredFromDag;
        return isDimmed
          ? {
              // Use an opaque wash instead of row opacity so sticky/group cells do not visually bleed.
              backgroundColor: 'color-mix(in srgb, hsl(var(--muted)) 55%, hsl(var(--card)))',
            }
          : {};
      }}
    />
  );
}
