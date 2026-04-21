// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import type { ColumnDef } from '@tanstack/react-table';
import { useAtomValue } from 'jotai';
import { GroupedDataTable } from './GroupedDataTable';
import { cn } from '@/lib/utils';
import type { AggMode, PivotedRow, HoveredStatInfo, PivotedStatTableSchema } from './types';
import type {
  GroupedDataTableGroupKeyEntry,
  GroupedDataTableSortInfo,
  DataHeaderProps,
  GroupCellProps,
  DataCellProps,
} from './types';
import {
  buildPivotedRows,
  expandRowsFromSchema,
  formatStatValue,
  formatNumericStat,
  getSchemaStatNames,
  getSortValue,
  gradientBg,
} from './utils';
import type {
  GroupedDataTableGroupRenderMode,
  GroupedDataTableVirtualizationOptions,
} from './GroupedDataTable';
import { nodeColorPaletteAtom } from '@/atoms/dag';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import type { ContinuousPaletteName } from '@/services/colors';
import { useColumnDragDrop } from './useColumnDragDrop';

const HIGHLIGHT_WASH = 'inset 0 0 0 999px hsl(var(--primary) / 0.07)';

function DataHeader({
  stat,
  sortInfo,
  onSort,
  className,
  style,
  rowHeaderHoverActive,
  draggedStat,
  getDropTargetPosition,
  onStatDragStart,
  onStatDragOver,
  onStatDragLeave,
  onStatDrop,
  onStatDragEnd,
  onHoverStat,
  buildHoveredStatInfo,
  hoveredStatName,
}: {
  stat: string;
  sortInfo: GroupedDataTableSortInfo | null;
  onSort: () => void;
  className?: string;
  style?: React.CSSProperties;
  rowHeaderHoverActive?: boolean;
  draggedStat: string | null;
  getDropTargetPosition?: (statName: string) => 'before' | 'after' | undefined;
  onStatDragStart: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragOver: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragLeave: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDrop: (e: React.DragEvent<HTMLTableCellElement>, statName: string) => void;
  onStatDragEnd: () => void;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  buildHoveredStatInfo: (statName: string) => HoveredStatInfo | null;
  hoveredStatName: string | undefined;
}) {
  const dropTargetPosition = getDropTargetPosition?.(stat);
  const dropTargetShadow =
    dropTargetPosition === 'before'
      ? 'inset 3px 0 0 hsl(var(--primary))'
      : dropTargetPosition === 'after'
        ? 'inset -3px 0 0 hsl(var(--primary))'
        : undefined;
  const mergedStyle =
    dropTargetShadow != null
      ? {
          ...style,
          boxShadow: style?.boxShadow
            ? `${style.boxShadow}, ${dropTargetShadow}`
            : dropTargetShadow,
        }
      : style;

  return (
    <th
      draggable
      onDragStart={e => onStatDragStart(e, stat)}
      onDragOver={e => onStatDragOver(e, stat)}
      onDragLeave={e => onStatDragLeave(e, stat)}
      onDrop={e => onStatDrop(e, stat)}
      onDragEnd={onStatDragEnd}
      onClick={() => {
        if (draggedStat !== null) return;
        onSort();
      }}
      onMouseEnter={() => onHoverStat?.(buildHoveredStatInfo(stat))}
      onMouseLeave={() => onHoverStat?.(null)}
      className={cn(
        'table-header-overlay text-right px-3 py-2 text-sm font-mono text-data whitespace-nowrap cursor-pointer select-none hover:text-foreground font-normal',
        className,
        {
          'font-semibold': Boolean(rowHeaderHoverActive),
          'opacity-50': draggedStat === stat,
          'text-foreground': Boolean(sortInfo),
          'table-header-overlay-active': hoveredStatName === stat || Boolean(rowHeaderHoverActive),
        }
      )}
      style={mergedStyle}
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
  onHoverStat,
  hoveredHeaderItemIds,
  setHoveredHeaderItemIds,
  hoveredItemId,
  selectedItemIds,
}: {
  row: PivotedRow;
  groupKey: GroupedDataTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  style?: React.CSSProperties;
  className?: string;
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  hoveredHeaderItemIds?: Set<string> | null;
  setHoveredHeaderItemIds?: (itemIds: Set<string> | null) => void;
  hoveredItemId?: string | null;
  selectedItemIds?: Set<string>;
  getGroupCellHandlers?: (
    groupKey: GroupedDataTableGroupKeyEntry,
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
  const hasDagSelection = (selectedItemIds?.size ?? 0) > 0;
  const isRowSelectedFromDag =
    hasDagSelection && [...row.itemIds].some(id => selectedItemIds?.has(id) === true);
  const backgroundColor = typeColor
    ? `color-mix(in srgb, ${typeColor} 15%, hsl(var(--card)))`
    : undefined;
  const accentColor = typeColor ? (isRowSelectedFromDag ? typeColor : backgroundColor) : undefined;
  const leftAccentShadow = accentColor ? `inset 8px 0 0 0 ${accentColor}` : undefined;
  const existingBoxShadow = style?.boxShadow;
  const combinedBoxShadow =
    leftAccentShadow && existingBoxShadow
      ? `${leftAccentShadow}, ${existingBoxShadow}`
      : (leftAccentShadow ?? existingBoxShadow);
  const baseStyle = typeColor
    ? {
        ...style,
        // Opaque tint so scrolled rows never bleed through sticky group cells.
        backgroundColor,
        boxShadow: combinedBoxShadow,
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
        onHoverStat?.(null);
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
      {formatNumericStat(displayVal, stat)}
    </td>
  );
}

interface PivotedStatTableProps<TRow> {
  rows: TRow[];
  schema: PivotedStatTableSchema<TRow>;
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
  virtualization?: GroupedDataTableVirtualizationOptions;
  groupRenderMode?: GroupedDataTableGroupRenderMode;
  stickyGroupColumns?: boolean;
  // display config
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  getGroupCellHandlers?: (
    groupKey: GroupedDataTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
  onReorderStat?: (from: string, to: string) => void;
}

export function PivotedStatTable<TRow>({
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
}: PivotedStatTableProps<TRow>) {
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  const rowRefs = useRef<Map<string, HTMLTableRowElement>>(new Map());
  const dragGhostRef = useRef<HTMLElement | null>(null);
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

  const removeDragGhost = useCallback(() => {
    if (dragGhostRef.current == null) return;
    dragGhostRef.current.remove();
    dragGhostRef.current = null;
  }, []);

  const createHeaderDragPreview = useCallback(
    (e: React.DragEvent<HTMLElement>) => {
      removeDragGhost();
      const header = e.currentTarget as HTMLTableCellElement;
      const rect = header.getBoundingClientRect();
      const offsetX = e.clientX - rect.left;
      const offsetY = e.clientY - rect.top;

      const dragGhost = header.cloneNode(true) as HTMLElement;
      dragGhost.style.position = 'fixed';
      dragGhost.style.top = '-1000px';
      dragGhost.style.left = '-1000px';
      dragGhost.style.width = `${rect.width}px`;
      dragGhost.style.pointerEvents = 'none';
      dragGhost.style.opacity = '0.95';
      dragGhost.style.border = '1px solid hsl(var(--primary) / 0.55)';
      dragGhost.style.borderRadius = '6px';
      dragGhost.style.backgroundColor = 'hsl(var(--card))';
      dragGhost.style.boxShadow = '0 8px 18px hsl(var(--foreground) / 0.22)';
      document.body.appendChild(dragGhost);
      dragGhostRef.current = dragGhost;
      e.dataTransfer.setDragImage(dragGhost, offsetX, offsetY);

      return () => {
        if (dragGhostRef.current === dragGhost) dragGhostRef.current = null;
        dragGhost.remove();
      };
    },
    [removeDragGhost]
  );

  const commitStatDrop = useCallback(
    (from: string, to: string, position: 'before' | 'after') => {
      setTableStatOrder(prev => {
        const next = prev.length > 0 ? [...prev] : [...resolvedVisibleStats];
        const fromIndex = next.indexOf(from);
        if (fromIndex < 0 || from === to) return next;
        const [moved] = next.splice(fromIndex, 1);
        const targetIndex = next.indexOf(to);
        if (targetIndex < 0) return next;
        const insertIndex = position === 'after' ? targetIndex + 1 : targetIndex;
        next.splice(insertIndex, 0, moved);
        return next;
      });
      // Keep optional callback for external listeners, without requiring global reordering.
      onReorderStat?.(from, to);
    },
    [onReorderStat, resolvedVisibleStats]
  );

  const statDragDrop = useColumnDragDrop({
    onDropCommit: commitStatDrop,
    createDragPreview: createHeaderDragPreview,
  });

  useEffect(() => {
    if (!hoveredItemId) return;
    const row = pivotedRows.find(r => r.itemIds.has(hoveredItemId));
    if (!row) return;
    const el = rowRefs.current.get(row.rowKey);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }, [hoveredItemId, pivotedRows]);

  const DataHeaderRenderer = useCallback(
    (props: DataHeaderProps) => (
      <DataHeader
        {...props}
        rowHeaderHoverActive={rowHeaderHoverActive}
        draggedStat={statDragDrop.draggedId}
        getDropTargetPosition={statDragDrop.getDropTargetPosition}
        onStatDragStart={statDragDrop.handleDragStart}
        onStatDragOver={statDragDrop.handleDragOver}
        onStatDragLeave={statDragDrop.handleDragLeave}
        onStatDrop={statDragDrop.handleDrop}
        onStatDragEnd={statDragDrop.handleDragEnd}
        onHoverStat={emitHoverStat}
        buildHoveredStatInfo={buildHoveredStatInfo}
        hoveredStatName={effectiveHoveredStat?.name}
      />
    ),
    [
      rowHeaderHoverActive,
      statDragDrop.draggedId,
      statDragDrop.getDropTargetPosition,
      statDragDrop.handleDragStart,
      statDragDrop.handleDragOver,
      statDragDrop.handleDragLeave,
      statDragDrop.handleDrop,
      statDragDrop.handleDragEnd,
      emitHoverStat,
      buildHoveredStatInfo,
      effectiveHoveredStat?.name,
    ]
  );

  const GroupCellRenderer = useCallback(
    (props: GroupCellProps<PivotedRow>) => (
      <GroupCell
        {...props}
        getGroupTypeColor={getGroupTypeColor}
        getGroupCellHandlers={getGroupCellHandlers}
        onHoverStat={emitHoverStat}
        hoveredHeaderItemIds={hoveredHeaderItemIds}
        setHoveredHeaderItemIds={setHoveredHeaderItemIds}
        hoveredItemId={hoveredItemId}
        selectedItemIds={selectedItemIds}
      />
    ),
    [
      getGroupTypeColor,
      getGroupCellHandlers,
      emitHoverStat,
      hoveredHeaderItemIds,
      hoveredItemId,
      selectedItemIds,
    ]
  );

  const DataCellRenderer = useCallback(
    (props: DataCellProps<PivotedRow>) => (
      <DataCell
        {...props}
        isAggregating={isAggregating}
        aggMode={aggMode}
        columnRanges={columnRanges}
        colorPalette={nodePalette}
        darkMode={isDarkMode}
        hoveredHeaderItemIds={hoveredHeaderItemIds}
        hoveredItemId={hoveredItemId}
        hoveredStatName={effectiveHoveredStat?.name}
        onHoverStat={emitHoverStat}
        buildHoveredStatInfo={buildHoveredStatInfo}
      />
    ),
    [
      isAggregating,
      aggMode,
      columnRanges,
      nodePalette,
      isDarkMode,
      hoveredHeaderItemIds,
      hoveredItemId,
      effectiveHoveredStat?.name,
      emitHoverStat,
      buildHoveredStatInfo,
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
    <GroupedDataTable
      data={pivotedRows}
      columns={columns}
      getRowId={row => row.rowKey}
      groupColumnIds={activeIndices}
      renderGroupHeader={columnId => (
        <span className={cn('relative z-10')}>{resolvedIndexLabels[columnId]}</span>
      )}
      DataHeader={DataHeaderRenderer}
      GroupCell={GroupCellRenderer}
      DataCell={DataCellRenderer}
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
