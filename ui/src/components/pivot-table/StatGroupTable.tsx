import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import type { ColumnDef } from '@tanstack/react-table';
import { PivotTable } from './PivotTable';
import { cn } from '@/lib/utils';
import type { AggMode, FlatRow, PivotedRow, HoveredStatInfo } from './types';
import type { PivotTableGroupKeyEntry, PivotTableSortInfo } from './types';
import {
  buildPivotedRows,
  formatStatValue,
  formatStatNumber,
  getSortValue,
  gradientBg,
  type GroupIndexDef,
} from './utils';

function DataHeader({
  stat,
  sortInfo,
  onSort,
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
        'text-right px-3 py-2 text-sm font-mono text-data whitespace-nowrap cursor-pointer select-none hover:text-foreground',
        draggedStat === stat && 'opacity-50',
        sortInfo && 'text-foreground',
        hoveredStatName === stat && 'bg-primary/10'
      )}
    >
      {stat}
      {sortInfo && <span className="ml-1 text-xs">{sortInfo.desc ? '▼' : '▲'}</span>}
    </th>
  );
}

function GroupCell({
  row,
  groupKey: gk,
  rowSpan,
  getGroupTypeColor,
  getGroupCellHandlers,
}: {
  row: PivotedRow;
  groupKey: PivotTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  getGroupCellHandlers?: (
    groupKey: PivotTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
}) {
  const typeColor = getGroupTypeColor?.(gk.key, gk.id);
  const handlers = getGroupCellHandlers?.(gk, row);

  return (
    <td
      className="px-3 py-1.5 whitespace-nowrap align-top border-r border-border/30"
      rowSpan={rowSpan}
      style={
        typeColor
          ? {
              borderLeftWidth: 8,
              borderLeftColor: typeColor,
              backgroundColor: `color-mix(in srgb, ${typeColor} 15%, transparent)`,
            }
          : undefined
      }
      onMouseEnter={handlers?.onMouseEnter}
      onMouseLeave={handlers?.onMouseLeave}
    >
      {gk.label}
    </td>
  );
}

function DataCell({
  row,
  stat,
  isAggregating,
  aggMode,
  columnRanges,
  hoveredStatName,
  onHoverStat,
  buildHoveredStatInfo,
}: {
  row: PivotedRow;
  stat: string;
  isAggregating: boolean;
  aggMode: AggMode;
  columnRanges: Map<string, { min: number; max: number }>;
  hoveredStatName: string | undefined;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  buildHoveredStatInfo: (statName: string) => HoveredStatInfo | null;
}) {
  const numVal = getSortValue(row, stat, isAggregating, aggMode);
  const range = columnRanges.get(stat);
  const bg = numVal !== null && range ? gradientBg(numVal, range.min, range.max) : undefined;
  const isStatHovered = hoveredStatName === stat;
  const colHighlight = isStatHovered ? 'inset 0 0 0 999px hsl(var(--primary) / 0.07)' : undefined;
  const statCellProps = {
    onMouseEnter: () => onHoverStat?.(buildHoveredStatInfo(stat)),
    onMouseLeave: () => onHoverStat?.(null),
  };
  if (!isAggregating) {
    const val = row.values.get(stat) ?? null;
    return (
      <td
        className="px-3 py-1.5 whitespace-nowrap text-right font-mono"
        style={{ backgroundColor: bg, boxShadow: colHighlight }}
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
        className="px-3 py-1.5 whitespace-nowrap text-right font-mono text-muted-foreground"
        style={{ boxShadow: colHighlight }}
        {...statCellProps}
      >
        -
      </td>
    );
  }
  const displayVal = agg[aggMode as Exclude<AggMode, 'value'>] ?? null;
  return (
    <td
      className="px-3 py-1.5 whitespace-nowrap text-right font-mono"
      style={{ backgroundColor: bg, boxShadow: colHighlight }}
      {...statCellProps}
    >
      {formatStatNumber(displayVal, stat)}
    </td>
  );
}

interface StatGroupTableProps {
  flatRows: FlatRow[];
  activeIndices: GroupIndexDef[];
  visibleStats: string[];
  isAggregating: boolean;
  aggMode: AggMode;
  indexLabels: Record<string, React.ReactNode>;
  // interaction state
  selectedItemIds?: Set<string>;
  hoveredItemId?: string | null;
  hoveredStat?: HoveredStatInfo | null;
  onHoverStat?: (info: HoveredStatInfo | null) => void;
  // display config
  getGroupTypeColor?: (key: string, id: string) => string | undefined;
  getGroupCellHandlers?: (
    groupKey: PivotTableGroupKeyEntry,
    row: PivotedRow
  ) => { onMouseEnter?: () => void; onMouseLeave?: () => void };
  onReorderStat?: (from: string, to: string) => void;
}

export function StatGroupTable({
  flatRows,
  activeIndices,
  visibleStats,
  isAggregating,
  aggMode,
  indexLabels,
  selectedItemIds,
  hoveredItemId,
  hoveredStat,
  onHoverStat,
  getGroupTypeColor,
  getGroupCellHandlers,
  onReorderStat,
}: StatGroupTableProps) {
  const rowRefs = useRef<Map<string, HTMLTableRowElement>>(new Map());
  const [draggedStat, setDraggedStat] = useState<string | null>(null);

  const statsByItem = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    for (const row of flatRows) {
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
  }, [flatRows]);

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
    () => buildPivotedRows(flatRows, activeIndices, isAggregating),
    [flatRows, activeIndices, isAggregating]
  );

  const columnRanges = useMemo(() => {
    const ranges = new Map<string, { min: number; max: number }>();
    for (const stat of visibleStats) {
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
  }, [pivotedRows, visibleStats, isAggregating, aggMode]);

  useEffect(() => {
    if (!hoveredItemId) return;
    const row = pivotedRows.find(r => r.itemIds.has(hoveredItemId));
    if (!row) return;
    const el = rowRefs.current.get(row.rowKey);
    el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, [hoveredItemId, pivotedRows]);

  const sharedProps = {
    draggedStat,
    setDraggedStat,
    onReorderStat,
    onHoverStat,
    buildHoveredStatInfo,
    hoveredStatName: hoveredStat?.name,
    getGroupTypeColor,
    getGroupCellHandlers,
    isAggregating,
    aggMode,
    columnRanges,
  };

  const columns = useMemo((): ColumnDef<PivotedRow>[] => {
    const groupCols: ColumnDef<PivotedRow>[] = activeIndices.map(def => ({
      id: def.key,
      header: String(indexLabels[def.key]),
      enableSorting: false,
    }));
    const statCols: ColumnDef<PivotedRow>[] = visibleStats.map(stat => ({
      id: stat,
      header: stat,
      enableSorting: true,
      accessorFn: (row: PivotedRow) =>
        getSortValue(row, stat, isAggregating, aggMode) ?? Number.NaN,
      sortingFn: (rowA, rowB, columnId) => {
        const a = getSortValue(rowA.original, columnId as string, isAggregating, aggMode);
        const b = getSortValue(rowB.original, columnId as string, isAggregating, aggMode);
        if (a === null && b === null) return 0;
        if (a === null) return 1;
        if (b === null) return -1;
        return a - b;
      },
    }));
    return [...groupCols, ...statCols];
  }, [activeIndices, visibleStats, indexLabels, isAggregating, aggMode]);

  const hasSelection = (selectedItemIds?.size ?? 0) > 0;
  const isSelected = (row: PivotedRow) => [...row.itemIds].some(id => selectedItemIds?.has(id));

  return (
    <PivotTable
      data={pivotedRows}
      columns={columns}
      getRowId={row => row.rowKey}
      groupColumnIds={activeIndices.map(def => def.key)}
      renderGroupHeader={columnId => indexLabels[columnId]}
      sharedProps={sharedProps}
      DataHeader={DataHeader}
      GroupCell={GroupCell}
      DataCell={DataCell}
      getRowRef={rowKey => el => {
        if (el) rowRefs.current.set(rowKey, el);
        else rowRefs.current.delete(rowKey);
      }}
      getRowClassName={row =>
        cn('border-b border-border/50 hover:bg-muted/50 transition-opacity', {
          'bg-muted/70': isSelected(row),
          'bg-primary/10':
            hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId),
        })
      }
      getRowStyle={row => {
        const isHoveredFromDag =
          hoveredItemId !== null && hoveredItemId !== undefined && row.itemIds.has(hoveredItemId);
        const isDimmed = hasSelection && !isSelected(row) && !isHoveredFromDag;
        return { opacity: isDimmed ? 0.3 : 1 };
      }}
    />
  );
}
