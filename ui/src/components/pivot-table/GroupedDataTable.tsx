// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { cn } from '@/lib/utils';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  type ColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { useVirtualizer } from '@tanstack/react-virtual';
import { computeRowSpans } from './utils';
import type {
  GroupedDataTableRowBase,
  GroupedDataTableSortInfo,
  GroupedDataTableGroupKeyEntry,
  DataHeaderProps,
  GroupCellProps,
  DataCellProps,
} from './types';

export interface GroupedDataTableVirtualizationOptions {
  enabled: boolean;
  estimateRowHeight?: number;
  overscan?: number;
}

export type GroupedDataTableGroupRenderMode = 'rowSpan' | 'compact';

export interface GroupedDataTableProps<TRow extends GroupedDataTableRowBase> {
  data: TRow[];
  columns: ColumnDef<TRow>[];
  getRowId: (row: TRow) => string;
  /** Column ids for group columns (rowSpan); must match order of row.groupKeys. */
  groupColumnIds: string[];
  renderToolbar?: React.ReactNode;
  renderGroupHeader?: (columnId: string) => React.ReactNode;
  DataHeader?: React.ComponentType<DataHeaderProps>;
  GroupCell?: React.ComponentType<GroupCellProps<TRow>>;
  DataCell?: React.ComponentType<DataCellProps<TRow>>;
  getRowRef?: (rowKey: string) => (el: HTMLTableRowElement | null) => void;
  getRowClassName?: (row: TRow) => string;
  getRowStyle?: (row: TRow) => React.CSSProperties;
  groupRenderMode?: GroupedDataTableGroupRenderMode;
  virtualization?: GroupedDataTableVirtualizationOptions;
  stickyGroupColumns?: boolean;
}

export function GroupedDataTable<TRow extends GroupedDataTableRowBase>({
  data,
  columns,
  getRowId,
  groupColumnIds,
  renderToolbar,
  renderGroupHeader,
  DataHeader,
  GroupCell,
  DataCell,
  getRowRef,
  getRowClassName,
  getRowStyle,
  groupRenderMode = 'rowSpan',
  virtualization,
  stickyGroupColumns = false,
}: GroupedDataTableProps<TRow>) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [groupColumnWidths, setGroupColumnWidths] = useState<number[]>([]);
  const groupHeaderRefs = useRef<Array<HTMLTableCellElement | null>>([]);

  const table = useReactTable({
    data,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getRowId: row => getRowId(row as TRow),
  });

  const tableRows = table.getRowModel().rows;
  const sortedRows = useMemo(() => tableRows.map(r => r.original as TRow), [tableRows]);
  const rowSpans = useMemo(
    () => (groupRenderMode === 'rowSpan' ? computeRowSpans(sortedRows) : []),
    [sortedRows, groupRenderMode]
  );

  const dataColumnIds = useMemo(
    () =>
      columns
        .map(c => (typeof c.id === 'string' ? c.id : String(c.id)))
        .filter(id => !groupColumnIds.includes(id)),
    [columns, groupColumnIds]
  );
  const stickyLeftOffsets = useMemo(() => {
    const offsets: number[] = [];
    let running = 0;
    for (let i = 0; i < groupColumnIds.length; i++) {
      offsets.push(running);
      running += groupColumnWidths[i] ?? 0;
    }
    return offsets;
  }, [groupColumnIds.length, groupColumnWidths]);
  const lastGroupColIndex = groupColumnIds.length - 1;

  useEffect(() => {
    if (!stickyGroupColumns) return;
    if (typeof ResizeObserver === 'undefined') return;
    const elements = groupHeaderRefs.current.filter((el): el is HTMLTableCellElement => el != null);
    if (elements.length === 0) return;
    const updateWidths = () => {
      setGroupColumnWidths(elements.map(el => el.getBoundingClientRect().width));
    };
    updateWidths();
    const observer = new ResizeObserver(updateWidths);
    for (const el of elements) observer.observe(el);
    window.addEventListener('resize', updateWidths);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', updateWidths);
    };
  }, [groupColumnIds, stickyGroupColumns]);

  const getStickyStyle = useCallback(
    (col: number, header: boolean): React.CSSProperties | undefined => {
      if (!stickyGroupColumns) return undefined;
      const style: React.CSSProperties = {
        position: 'sticky',
        left: stickyLeftOffsets[col] ?? 0,
        // Keep sticky group headers/cells above scrollable data cells for both paint and hit-testing.
        zIndex: header ? 90 : 70,
        backgroundColor: 'hsl(var(--card))',
        pointerEvents: 'auto',
      };
      if (header) {
        style.top = 0;
      }
      if (col === lastGroupColIndex) {
        style.boxShadow = '2px 0 0 hsl(var(--border) / 0.6)';
      }
      return style;
    },
    [stickyGroupColumns, stickyLeftOffsets, lastGroupColIndex]
  );

  const virtualizationEnabled = Boolean(virtualization?.enabled);
  const rowVirtualizer = useVirtualizer({
    count: sortedRows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => virtualization?.estimateRowHeight ?? 34,
    overscan: virtualization?.overscan ?? 10,
    enabled: virtualizationEnabled,
  });
  const virtualRows = virtualizationEnabled ? rowVirtualizer.getVirtualItems() : [];
  const topPadding = virtualizationEnabled && virtualRows.length > 0 ? virtualRows[0]!.start : 0;
  const bottomPadding =
    virtualizationEnabled && virtualRows.length > 0
      ? rowVirtualizer.getTotalSize() - virtualRows[virtualRows.length - 1]!.end
      : 0;

  const rowsToRender = virtualizationEnabled
    ? virtualRows.map(vRow => ({ row: sortedRows[vRow.index]!, rowIndex: vRow.index }))
    : sortedRows.map((row, rowIndex) => ({ row, rowIndex }));

  const totalColumnCount = groupColumnIds.length + dataColumnIds.length;
  const normalHeaderStyle: React.CSSProperties = {
    position: 'sticky',
    top: 0,
    zIndex: 20,
    backgroundColor: 'hsl(var(--card))',
  };

  return (
    <div className="flex flex-col h-full">
      {renderToolbar != null && (
        <div className="shrink-0 flex flex-col border-b border-border bg-card">{renderToolbar}</div>
      )}
      <div className="flex-1 min-h-0 overflow-auto" ref={scrollRef}>
        <table className="text-xs border-separate border-spacing-0 relative isolate">
          <thead className="bg-card">
            <tr className="border-b border-border">
              {groupColumnIds.map(columnId => (
                <th
                  key={columnId}
                  ref={el => {
                    const idx = groupColumnIds.indexOf(columnId);
                    if (idx >= 0) groupHeaderRefs.current[idx] = el;
                  }}
                  className={cn(
                    'table-header-overlay text-left px-3 py-2 text-sm text-muted-foreground whitespace-nowrap font-normal'
                  )}
                  style={getStickyStyle(groupColumnIds.indexOf(columnId), true)}
                >
                  {renderGroupHeader?.(columnId) ?? columnId}
                </th>
              ))}
              {dataColumnIds.map(columnId => {
                const sortEntry = sorting.find(s => s.id === columnId);
                const sortInfo: GroupedDataTableSortInfo | null =
                  sortEntry != null ? { desc: sortEntry.desc } : null;
                const onSort = () => {
                  setSorting(prev => {
                    const current = prev.find(s => s.id === columnId);
                    if (!current) return [{ id: columnId, desc: true }];
                    if (current.desc) return [{ id: columnId, desc: false }];
                    return [];
                  });
                };
                return (
                  <React.Fragment key={columnId}>
                    {DataHeader ? (
                      <DataHeader
                        stat={columnId}
                        sortInfo={sortInfo}
                        onSort={onSort}
                        className="bg-card"
                        style={normalHeaderStyle}
                      />
                    ) : (
                      <th
                        className="text-right px-3 py-2 text-sm whitespace-nowrap bg-card"
                        style={normalHeaderStyle}
                        onClick={onSort}
                      >
                        {columnId}
                      </th>
                    )}
                  </React.Fragment>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {virtualizationEnabled && topPadding > 0 && (
              <tr>
                <td
                  colSpan={totalColumnCount}
                  style={{ height: topPadding, padding: 0, border: 0 }}
                />
              </tr>
            )}
            {rowsToRender.map(({ row, rowIndex }) => {
              const spans = rowSpans[rowIndex];
              const prevRow = rowIndex > 0 ? sortedRows[rowIndex - 1] : null;
              return (
                <tr
                  key={row.rowKey}
                  ref={getRowRef?.(row.rowKey)}
                  className={getRowClassName?.(row)}
                  style={getRowStyle?.(row)}
                >
                  {groupColumnIds.map((_, col) => {
                    if (groupRenderMode === 'rowSpan') {
                      if (spans == null || spans[col] == null || row.groupKeys[col] == null)
                        return null;
                      return (
                        <React.Fragment key={col}>
                          {GroupCell && (
                            <GroupCell
                              row={row as TRow}
                              groupKey={row.groupKeys[col] as GroupedDataTableGroupKeyEntry}
                              rowSpan={spans[col]!}
                              columnIndex={col}
                              style={getStickyStyle(col, false)}
                            />
                          )}
                        </React.Fragment>
                      );
                    }

                    const groupKey = row.groupKeys[col] as
                      | GroupedDataTableGroupKeyEntry
                      | undefined;
                    if (!groupKey) return null;
                    const isRepeated =
                      prevRow != null &&
                      row.groupKeys
                        .slice(0, col + 1)
                        .every((gk, gkIndex) => gk.id === prevRow.groupKeys[gkIndex]?.id);
                    const displayGroupKey = isRepeated ? { ...groupKey, label: '' } : groupKey;
                    return (
                      <React.Fragment key={col}>
                        {GroupCell && (
                          <GroupCell
                            row={row as TRow}
                            groupKey={displayGroupKey}
                            rowSpan={1}
                            columnIndex={col}
                            style={getStickyStyle(col, false)}
                          />
                        )}
                      </React.Fragment>
                    );
                  })}
                  {dataColumnIds.map(columnId => (
                    <React.Fragment key={columnId}>
                      {DataCell && <DataCell row={row as TRow} stat={columnId} />}
                    </React.Fragment>
                  ))}
                </tr>
              );
            })}
            {virtualizationEnabled && bottomPadding > 0 && (
              <tr>
                <td
                  colSpan={totalColumnCount}
                  className="border-0 p-0"
                  style={{ height: bottomPadding }}
                />
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
