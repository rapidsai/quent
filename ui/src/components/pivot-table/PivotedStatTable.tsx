// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import type { ColumnDef, OnChangeFn, SortingState } from '@tanstack/react-table';
import { useAtomValue } from 'jotai';
import { GroupedDataTable } from './GroupedDataTable';
import { cn } from '@/lib/utils';
import type { AggMode, PivotedRow, HoveredStatInfo, PivotedStatTableSchema } from './types';
import type {
  DataHeaderProps,
  GroupCellProps,
  DataCellProps,
  PivotTableInteractionConfig,
  PivotTableRenderConfig,
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
import { nodeColorPaletteAtom } from '@/atoms/dag';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import { useColumnDragDrop } from './useColumnDragDrop';
import {
  PivotTableRenderProvider,
  usePivotTableRenderContext,
  type PivotTableRenderContextValue,
} from './PivotTableRenderContext';

const HIGHLIGHT_WASH = 'inset 0 0 0 999px hsl(var(--primary) / 0.07)';

function DataHeader({ stat, sortInfo, onSort, className, style }: DataHeaderProps) {
  const { dnd, interaction, derived } = usePivotTableRenderContext();
  const hoveredStatName = interaction.hoveredStat?.name;
  // While a column is being dragged, show a 3px primary-colored bar on the
  // leading or trailing edge of the column under the cursor to indicate where
  // a drop would insert.
  const dropTargetPosition = dnd.getDropTargetPosition?.(stat);
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
      onDragStart={e => dnd.onStatDragStart(e, stat)}
      onDragOver={e => dnd.onStatDragOver(e, stat)}
      onDragLeave={e => dnd.onStatDragLeave(e, stat)}
      onDrop={e => dnd.onStatDrop(e, stat)}
      onDragEnd={dnd.onStatDragEnd}
      onClick={() => {
        if (dnd.draggedStat !== null) return;
        onSort();
      }}
      onMouseEnter={() => interaction.setHoveredStat(derived.buildHoveredStatInfo(stat))}
      onMouseLeave={() => interaction.setHoveredStat(null)}
      className={cn(
        'table-header-overlay text-right px-3 py-2 text-sm font-mono text-data whitespace-nowrap cursor-pointer select-none hover:text-foreground font-normal',
        className,
        {
          'font-semibold': Boolean(sortInfo),
          'opacity-50': dnd.draggedStat === stat,
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
  columnIndex: _columnIndex,
  style,
  className,
}: GroupCellProps<PivotedRow>) {
  const { interaction, renderConfig } = usePivotTableRenderContext();
  const typeColor = renderConfig.getGroupTypeColor?.(gk.key, gk.id);
  const handlers = interaction.groupCellHandlers?.(gk, row);
  const isRowHighlightedFromDag =
    interaction.hoveredItemId !== null &&
    interaction.hoveredItemId !== undefined &&
    row.itemIds.has(interaction.hoveredItemId);
  const hasDagSelection = (interaction.selectedItemIds?.size ?? 0) > 0;
  const isRowSelectedFromDag =
    hasDagSelection &&
    interaction.selectedItemIds != null &&
    itemHasId(row.itemIds, interaction.selectedItemIds);
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
        interaction.setHoveredStat(null);
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

function DataCell({ row, stat }: DataCellProps<PivotedRow>) {
  const { display, interaction, derived } = usePivotTableRenderContext();
  const numVal = getSortValue(row, stat, display.isAggregating, display.aggMode);
  const range = derived.columnRanges.get(stat);
  const bg =
    numVal !== null && range
      ? gradientBg(numVal, range.min, range.max, display.colorPalette, display.darkMode)
      : undefined;
  const isStatHovered = interaction.hoveredStat?.name === stat;
  const colHighlight = isStatHovered ? HIGHLIGHT_WASH : undefined;
  const isRowHighlightedFromTable =
    derived.hoveredHeaderItemIds != null && itemHasId(row.itemIds, derived.hoveredHeaderItemIds);
  const isRowHighlightedFromDag =
    interaction.hoveredItemId !== null &&
    interaction.hoveredItemId !== undefined &&
    row.itemIds.has(interaction.hoveredItemId);
  const rowHighlight =
    isRowHighlightedFromTable || isRowHighlightedFromDag ? HIGHLIGHT_WASH : undefined;
  const cellHighlight = rowHighlight ?? colHighlight;
  const statCellProps = {
    onMouseEnter: () => interaction.setHoveredStat(derived.buildHoveredStatInfo(stat)),
    onMouseLeave: () => interaction.setHoveredStat(null),
  };
  if (!display.isAggregating) {
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
  const displayVal = agg[display.aggMode as Exclude<AggMode, 'value'>] ?? null;
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
  interaction: PivotTableInteractionConfig<PivotedRow>;
  renderConfig?: PivotTableRenderConfig;
  virtualization?: GroupedDataTableVirtualizationOptions;
  groupRenderMode?: GroupedDataTableGroupRenderMode;
  stickyGroupColumns?: boolean;
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
  interaction,
  renderConfig,
  virtualization,
  groupRenderMode,
  stickyGroupColumns = true,
  onReorderStat,
  sorting,
  onSortingChange,
}: PivotedStatTableProps<TRow>) {
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  const rowRefs = useRef<Map<string, HTMLTableRowElement>>(new Map());
  const dragGhostRef = useRef<HTMLElement | null>(null);
  const [hoveredHeaderItemIds, setHoveredHeaderItemIds] = useState<Set<string> | null>(null);
  const [tableStatOrder, setTableStatOrder] = useState<string[]>([]);
  const setHoveredStat = interaction.setHoveredStat;
  const effectiveHoveredStat = interaction.hoveredStat;
  const effectiveHoveredItemId = interaction.hoveredItemId;
  const effectiveSelectedItemIds = interaction.selectedItemIds;
  const effectiveGroupCellHandlers = interaction.groupCellHandlers;
  const effectiveOnTableMouseLeave = interaction.onTableMouseLeave;
  const effectiveRenderConfig = useMemo(
    (): PivotTableRenderConfig => ({
      getGroupTypeColor: renderConfig?.getGroupTypeColor,
    }),
    [renderConfig]
  );
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
    setHoveredStat(null);
    effectiveOnTableMouseLeave?.();
  }, [setHoveredStat, effectiveOnTableMouseLeave]);

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
    if (!effectiveHoveredItemId) return;
    const row = visiblePivotedRows.find(r => r.itemIds.has(effectiveHoveredItemId));
    if (!row) return;
    const el = rowRefs.current.get(row.rowKey);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }, [effectiveHoveredItemId, visiblePivotedRows]);

  const interactionContextValue = useMemo(
    () => ({
      hoveredStat: effectiveHoveredStat,
      setHoveredStat,
      hoveredItemId: effectiveHoveredItemId,
      selectedItemIds: effectiveSelectedItemIds,
      onTableMouseLeave: effectiveOnTableMouseLeave,
      groupCellHandlers: effectiveGroupCellHandlers,
    }),
    [
      effectiveHoveredStat,
      setHoveredStat,
      effectiveHoveredItemId,
      effectiveSelectedItemIds,
      effectiveOnTableMouseLeave,
      effectiveGroupCellHandlers,
    ]
  );
  const displayContextValue = useMemo(
    () => ({
      isAggregating,
      aggMode,
      colorPalette: nodePalette,
      darkMode: isDarkMode,
    }),
    [isAggregating, aggMode, nodePalette, isDarkMode]
  );
  const dndContextValue = useMemo(
    () => ({
      draggedStat: statDragDrop.draggedId,
      getDropTargetPosition: statDragDrop.getDropTargetPosition,
      onStatDragStart: statDragDrop.handleDragStart,
      onStatDragOver: statDragDrop.handleDragOver,
      onStatDragLeave: statDragDrop.handleDragLeave,
      onStatDrop: statDragDrop.handleDrop,
      onStatDragEnd: statDragDrop.handleDragEnd,
    }),
    [statDragDrop]
  );
  const derivedContextValue = useMemo(
    () => ({
      hoveredHeaderItemIds,
      columnRanges,
      buildHoveredStatInfo,
    }),
    [hoveredHeaderItemIds, columnRanges, buildHoveredStatInfo]
  );
  const renderContextValue = useMemo(
    (): PivotTableRenderContextValue => ({
      interaction: interactionContextValue,
      renderConfig: effectiveRenderConfig,
      display: displayContextValue,
      dnd: dndContextValue,
      derived: derivedContextValue,
    }),
    [
      interactionContextValue,
      effectiveRenderConfig,
      displayContextValue,
      dndContextValue,
      derivedContextValue,
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

  const hasSelection = (effectiveSelectedItemIds?.size ?? 0) > 0;
  const isSelected = (row: PivotedRow) =>
    effectiveSelectedItemIds != null && itemHasId(row.itemIds, effectiveSelectedItemIds);
  const renderGroupHeader = useCallback(
    (columnId: string) => (
      <span className={cn('relative z-10')}>{resolvedIndexLabels[columnId]}</span>
    ),
    [resolvedIndexLabels]
  );

  const getRowRef = useCallback(
    (rowKey: string) => (el: HTMLTableRowElement | null) => {
      if (el) rowRefs.current.set(rowKey, el);
      else rowRefs.current.delete(rowKey);
    },
    []
  );

  return (
    <div
      className="h-full"
      onMouseLeave={handleTableMouseLeave}
      onPointerLeave={handleTableMouseLeave}
    >
      <PivotTableRenderProvider value={renderContextValue}>
        <GroupedDataTable
          data={visiblePivotedRows}
          columns={columns}
          getRowId={row => row.rowKey}
          groupColumnIds={activeIndices}
          renderGroupHeader={renderGroupHeader}
          DataHeader={DataHeader}
          GroupCell={GroupCell}
          DataCell={DataCell}
          virtualization={virtualization}
          groupRenderMode={effectiveGroupRenderMode}
          stickyGroupColumns={stickyGroupColumns}
          sorting={sorting}
          onSortingChange={onSortingChange}
          getRowRef={getRowRef}
          getRowClassName={row =>
            cn('border-b border-border/50 hover:bg-muted/50 transition-opacity', {
              'bg-muted/70': isSelected(row),
            })
          }
          getRowStyle={row => {
            const isHoveredFromDag =
              effectiveHoveredItemId !== null &&
              effectiveHoveredItemId !== undefined &&
              row.itemIds.has(effectiveHoveredItemId);
            const isDimmed = hasSelection && !isSelected(row) && !isHoveredFromDag;
            return isDimmed
              ? {
                  // Use an opaque wash instead of row opacity so sticky/group cells do not visually bleed.
                  backgroundColor: 'color-mix(in srgb, hsl(var(--muted)) 55%, hsl(var(--card)))',
                }
              : {};
          }}
        />
      </PivotTableRenderProvider>
    </div>
  );
}
