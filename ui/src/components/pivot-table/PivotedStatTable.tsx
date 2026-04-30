// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import type { ColumnDef, OnChangeFn, SortingState } from '@tanstack/react-table';
import { GroupedDataTable } from './GroupedDataTable';
import { cn } from '@quent/utils';
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
  itemHasId,
} from './utils';
import type {
  GroupedDataTableGroupRenderMode,
  GroupedDataTableVirtualizationOptions,
} from './GroupedDataTable';
import { useNodeColorPalette } from '@quent/hooks';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import type { ContinuousPaletteName } from '@quent/utils';
import { useColumnDragDrop } from './useColumnDragDrop';

const HIGHLIGHT_WASH = 'inset 0 0 0 999px hsl(var(--primary) / 0.07)';

/**
 * Stable wrapper for renderer components passed into `<GroupedDataTable>`.
 *
 * The naive pattern — `const X = useCallback((props) => <Inner {...props} foo={foo} />, [foo])` —
 * returns a new function reference whenever `foo` changes, so React sees a
 * different component *type* at the same JSX position and unmounts/remounts
 * the underlying DOM. For our hover handlers that's fatal: the `<td>`'s
 * `mouseleave` handler is removed before the user can leave the cell, so the
 * highlight set by `mouseenter` is never paired with a `mouseleave` and the
 * DAG ends up with orphan-highlighted nodes (and stale stat heatmap colors).
 *
 * This hook holds the latest impl in a ref and exposes a single memoised
 * invocation function, so the wrapper reference is permanently stable while
 * each call still picks up the latest closure values.
 */
function useStableRenderer<P>(impl: (props: P) => React.ReactNode): (props: P) => React.ReactNode {
  const implRef = useRef(impl);
  implRef.current = impl;
  return useMemo(() => (props: P) => implRef.current(props), []);
}

function DataHeader({
  stat,
  sortInfo,
  onSort,
  className,
  style,
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
  // While a column is being dragged, show a 3px primary-colored bar on the
  // leading or trailing edge of the column under the cursor to indicate where
  // a drop would insert.
  const dropTargetPosition = getDropTargetPosition?.(stat);
  let dropTargetShadow: string | undefined;
  if (dropTargetPosition === 'before') {
    dropTargetShadow = 'inset 3px 0 0 hsl(var(--primary))';
  } else if (dropTargetPosition === 'after') {
    dropTargetShadow = 'inset -3px 0 0 hsl(var(--primary))';
  }

  // The drop indicator must layer on top of any existing box-shadow (e.g. the
  // sticky/group-cell accent), not replace it.
  const mergedStyle: React.CSSProperties | undefined = dropTargetShadow
    ? {
        ...style,
        boxShadow: [style?.boxShadow, dropTargetShadow].filter(Boolean).join(', '),
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
          'font-semibold': Boolean(sortInfo),
          'opacity-50': draggedStat === stat,
          'text-foreground': Boolean(sortInfo),
          'table-header-overlay-active': hoveredStatName === stat,
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
  hoveredItemId?: string | null;
  selectedItemIds?: Set<string>;
  getGroupCellHandlers?: (
    groupKey: GroupedDataTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
}) {
  const typeColor = getGroupTypeColor?.(gk.key, gk.id);
  const handlers = getGroupCellHandlers?.(gk, row);
  const isRowHighlightedFromDag =
    hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId);
  const hasDagSelection = (selectedItemIds?.size ?? 0) > 0;
  const isRowSelectedFromDag =
    hasDagSelection && selectedItemIds != null && itemHasId(row.itemIds, selectedItemIds);
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

  return (
    <td
      className={cn(
        'table-header-overlay px-3 py-1.5 whitespace-nowrap align-top border-r border-border/30',
        isRowHighlightedFromDag && 'table-header-overlay-active',
        className
      )}
      rowSpan={rowSpan}
      style={baseStyle}
      onMouseEnter={() => {
        onHoverStat?.(null);
        handlers?.onMouseEnter?.();
      }}
      onMouseLeave={() => {
        handlers?.onMouseLeave?.();
      }}
      onPointerLeave={() => {
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
    hoveredHeaderItemIds != null && itemHasId(row.itemIds, hoveredHeaderItemIds);
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
  /**
   * If true, rows with no value for any visible stat (e.g. an operator/plan
   * that doesn't produce any of the currently selected statistics) are
   * filtered out before rendering. Defaults to true. Flip to false to expose
   * a future "show null rows" toggle.
   */
  hideEmptyRows?: boolean;
  // interaction state
  selectedItemIds?: Set<string>;
  hoveredItemId?: string | null;
  hoveredStat?: HoveredStatInfo | null;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  onTableMouseLeave?: () => void;
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
  /** Optional controlled sort state, forwarded to the underlying GroupedDataTable. */
  sorting?: SortingState;
  onSortingChange?: OnChangeFn<SortingState>;
}

export function PivotedStatTable<TRow>({
  rows,
  schema,
  activeIndices,
  visibleStats,
  isAggregating = false,
  aggMode = 'sum',
  indexLabels,
  hideEmptyRows = true,
  selectedItemIds,
  hoveredItemId,
  hoveredStat,
  onHoverStat,
  onTableMouseLeave,
  virtualization,
  groupRenderMode,
  stickyGroupColumns = true,
  getGroupTypeColor,
  getGroupCellHandlers,
  onReorderStat,
  sorting,
  onSortingChange,
}: PivotedStatTableProps<TRow>) {
  const [nodePalette] = useNodeColorPalette();
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

  useEffect(() => {
    setTableStatOrder(prev => {
      if (prev.length === 0) return resolvedVisibleStats;
      const visibleSet = new Set(resolvedVisibleStats);
      const kept = prev.filter(stat => visibleSet.has(stat));
      const additions = resolvedVisibleStats.filter(stat => !kept.includes(stat));
      return [...kept, ...additions];
    });
  }, [resolvedVisibleStats]);

  const effectiveVisibleStats = tableStatOrder.length > 0 ? tableStatOrder : resolvedVisibleStats;
  const resolvedIndexLabels = useMemo(
    () =>
      indexLabels ??
      (Object.fromEntries(activeIndices.map(key => [key, key])) as Record<string, React.ReactNode>),
    [activeIndices, indexLabels]
  );
  const effectiveGroupRenderMode =
    groupRenderMode ?? (virtualization?.enabled ? 'compact' : 'rowSpan');

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

  // A pivoted row is "empty" when none of the currently visible stats have a
  // renderable value for it. In non-aggregating mode that means every
  // `values.get(stat)` is null/undefined; in aggregating mode it means every
  // `aggs.get(stat)` is missing or non-numeric (i.e. the cell would render '-').
  const visiblePivotedRows = useMemo(() => {
    if (!hideEmptyRows) return pivotedRows;
    return pivotedRows.filter(row => {
      for (const stat of effectiveVisibleStats) {
        if (isAggregating) {
          const agg = row.aggs.get(stat);
          if (agg && agg.isNumeric) return true;
        } else {
          const v = row.values.get(stat);
          if (v !== null && v !== undefined) return true;
        }
      }
      return false;
    });
  }, [pivotedRows, hideEmptyRows, effectiveVisibleStats, isAggregating]);

  const columnRanges = useMemo(() => {
    const ranges = new Map<string, { min: number; max: number }>();
    for (const stat of effectiveVisibleStats) {
      let min = Infinity;
      let max = -Infinity;
      for (const row of visiblePivotedRows) {
        const v = getSortValue(row, stat, isAggregating, aggMode);
        if (v !== null) {
          if (v < min) min = v;
          if (v > max) max = v;
        }
      }
      if (min !== Infinity) ranges.set(stat, { min, max });
    }
    return ranges;
  }, [visiblePivotedRows, effectiveVisibleStats, isAggregating, aggMode]);

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

      // The native HTML5 drag API requires a real DOM node already in the
      // document for `setDragImage`. Cloning the source header gives us a
      // pixel-perfect preview without re-rendering it via React.
      const dragGhost = header.cloneNode(true) as HTMLElement;
      Object.assign(dragGhost.style, {
        position: 'fixed',
        top: '-1000px',
        left: '-1000px',
        width: `${rect.width}px`,
        pointerEvents: 'none',
        opacity: '0.95',
        border: '1px solid hsl(var(--primary) / 0.55)',
        borderRadius: '4px',
        backgroundColor: 'hsl(var(--card))',
      } satisfies Partial<CSSStyleDeclaration>);
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

  const handleTableMouseLeave = useCallback(() => {
    setHoveredHeaderItemIds(null);
    emitHoverStat(null);
    onTableMouseLeave?.();
  }, [emitHoverStat, onTableMouseLeave]);

  // Safety net: fast mouse movements, drag captures, and pointer-events: none
  // overlays can all swallow the per-element onMouseLeave. Clear stale hover
  // state whenever the pointer exits the document or the window loses focus.
  useEffect(() => {
    const onDocPointerOut = (e: PointerEvent) => {
      if (e.relatedTarget == null) handleTableMouseLeave();
    };
    const onWindowBlur = () => handleTableMouseLeave();
    const onVisibilityChange = () => {
      if (document.visibilityState !== 'visible') handleTableMouseLeave();
    };
    document.addEventListener('pointerout', onDocPointerOut);
    window.addEventListener('blur', onWindowBlur);
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      document.removeEventListener('pointerout', onDocPointerOut);
      window.removeEventListener('blur', onWindowBlur);
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [handleTableMouseLeave]);

  useEffect(() => {
    if (!hoveredItemId) return;
    const row = visiblePivotedRows.find(r => r.itemIds.has(hoveredItemId));
    if (!row) return;
    const el = rowRefs.current.get(row.rowKey);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }, [hoveredItemId, visiblePivotedRows]);

  // The renderers below are intentionally NOT wrapped in `useCallback`. A
  // changing useCallback dep list would re-create the function reference and
  // force <GroupedDataTable> to remount every cell on every atom update,
  // dropping the in-flight mouseleave handlers — see `useStableRenderer`.
  const DataHeaderRenderer = useStableRenderer<DataHeaderProps>(props => (
    <DataHeader
      {...props}
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
  ));

  const GroupCellRenderer = useStableRenderer<GroupCellProps<PivotedRow>>(props => (
    <GroupCell
      {...props}
      getGroupTypeColor={getGroupTypeColor}
      getGroupCellHandlers={getGroupCellHandlers}
      onHoverStat={emitHoverStat}
      hoveredItemId={hoveredItemId}
      selectedItemIds={selectedItemIds}
    />
  ));

  const DataCellRenderer = useStableRenderer<DataCellProps<PivotedRow>>(props => (
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
  ));

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
  const isSelected = (row: PivotedRow) =>
    selectedItemIds != null && itemHasId(row.itemIds, selectedItemIds);

  return (
    <div
      className="h-full"
      onMouseLeave={handleTableMouseLeave}
      onPointerLeave={handleTableMouseLeave}
    >
      <GroupedDataTable
        data={visiblePivotedRows}
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
        sorting={sorting}
        onSortingChange={onSortingChange}
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
    </div>
  );
}
