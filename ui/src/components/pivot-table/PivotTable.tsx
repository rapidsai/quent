import React, { useMemo, useState } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  type ColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { computeRowSpans } from './utils';
import type { PivotTableRowBase, PivotTableSortInfo } from './types';

export interface PivotTableProps<TRow extends PivotTableRowBase> {
  data: TRow[];
  columns: ColumnDef<TRow>[];
  getRowId: (row: TRow) => string;
  /** Column ids for group columns (rowSpan); must match order of row.groupKeys. */
  groupColumnIds: string[];
  renderToolbar?: React.ReactNode;
  renderGroupHeader?: (columnId: string) => React.ReactNode;
  renderDataHeader?: (
    columnId: string,
    sortInfo: PivotTableSortInfo | null,
    onSort: () => void
  ) => React.ReactNode;
  renderGroupCell?: (
    row: TRow,
    groupKeyEntry: { key: string; id: string; label: string },
    rowSpan: number,
    columnIndex: number
  ) => React.ReactNode;
  renderDataCell?: (row: TRow, columnId: string) => React.ReactNode;
  getRowRef?: (rowKey: string) => (el: HTMLTableRowElement | null) => void;
  getRowClassName?: (row: TRow) => string;
  getRowStyle?: (row: TRow) => React.CSSProperties;
}

export function PivotTable<TRow extends PivotTableRowBase>({
  data,
  columns,
  getRowId,
  groupColumnIds,
  renderToolbar,
  renderGroupHeader,
  renderDataHeader,
  renderGroupCell,
  renderDataCell,
  getRowRef,
  getRowClassName,
  getRowStyle,
}: PivotTableProps<TRow>) {
  const [sorting, setSorting] = useState<SortingState>([]);

  const table = useReactTable({
    data,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getRowId: row => getRowId(row as TRow),
  });

  const sortedRows = useMemo(
    () => table.getRowModel().rows.map(r => r.original as TRow),
    [table.getRowModel().rows]
  );
  const rowSpans = useMemo(() => computeRowSpans(sortedRows), [sortedRows]);

  const dataColumnIds = useMemo(
    () =>
      columns
        .map(c => (typeof c.id === 'string' ? c.id : String(c.id)))
        .filter(id => !groupColumnIds.includes(id)),
    [columns, groupColumnIds]
  );

  return (
    <div className="flex flex-col h-full">
      {renderToolbar != null && (
        <div className="shrink-0 flex flex-col border-b border-border bg-card">{renderToolbar}</div>
      )}
      <div className="flex-1 min-h-0 overflow-auto">
        <table className="text-sm border-collapse">
          <thead className="sticky top-0 bg-card z-10">
            <tr className="border-b border-border">
              {groupColumnIds.map(columnId => (
                <th
                  key={columnId}
                  className="text-left px-3 py-2 font-medium text-muted-foreground whitespace-nowrap"
                >
                  {renderGroupHeader?.(columnId) ?? columnId}
                </th>
              ))}
              {dataColumnIds.map(columnId => {
                const sortEntry = sorting.find(s => s.id === columnId);
                const sortInfo: PivotTableSortInfo | null =
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
                    {renderDataHeader?.(columnId, sortInfo, onSort) ?? (
                      <th
                        className="text-right px-3 py-2 font-medium whitespace-nowrap"
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
            {sortedRows.map((row, i) => {
              const spans = rowSpans[i];
              return (
                <tr
                  key={row.rowKey}
                  ref={getRowRef?.(row.rowKey)}
                  className={getRowClassName?.(row)}
                  style={getRowStyle?.(row)}
                >
                  {groupColumnIds.map((_, col) =>
                    spans != null && spans[col] != null && row.groupKeys[col] != null ? (
                      <React.Fragment key={col}>
                        {renderGroupCell?.(
                          row,
                          row.groupKeys[col] as { key: string; id: string; label: string },
                          spans[col]!,
                          col
                        )}
                      </React.Fragment>
                    ) : null
                  )}
                  {dataColumnIds.map(columnId => (
                    <React.Fragment key={columnId}>
                      {renderDataCell?.(row, columnId)}
                    </React.Fragment>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
