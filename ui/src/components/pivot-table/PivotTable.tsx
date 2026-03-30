import React, { useMemo, useState } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  type ColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { computeRowSpans } from './utils';
import type {
  PivotTableRowBase,
  PivotTableSortInfo,
  PivotTableGroupKeyEntry,
  DataHeaderProps,
  GroupCellProps,
  DataCellProps,
} from './types';

export interface PivotTableProps<
  TRow extends PivotTableRowBase,
  TShared extends object = Record<never, never>,
> {
  data: TRow[];
  columns: ColumnDef<TRow>[];
  getRowId: (row: TRow) => string;
  /** Column ids for group columns (rowSpan); must match order of row.groupKeys. */
  groupColumnIds: string[];
  renderToolbar?: React.ReactNode;
  renderGroupHeader?: (columnId: string) => React.ReactNode;
  /** Extra props forwarded verbatim to every DataHeader, GroupCell and DataCell instance. */
  sharedProps?: TShared;
  DataHeader?: React.ComponentType<DataHeaderProps & TShared>;
  GroupCell?: React.ComponentType<GroupCellProps<TRow> & TShared>;
  DataCell?: React.ComponentType<DataCellProps<TRow> & TShared>;
  getRowRef?: (rowKey: string) => (el: HTMLTableRowElement | null) => void;
  getRowClassName?: (row: TRow) => string;
  getRowStyle?: (row: TRow) => React.CSSProperties;
}

export function PivotTable<
  TRow extends PivotTableRowBase,
  TShared extends object = Record<never, never>,
>({
  data,
  columns,
  getRowId,
  groupColumnIds,
  renderToolbar,
  renderGroupHeader,
  sharedProps,
  DataHeader,
  GroupCell,
  DataCell,
  getRowRef,
  getRowClassName,
  getRowStyle,
}: PivotTableProps<TRow, TShared>) {
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

  const shared = (sharedProps ?? {}) as TShared;

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
                  className="text-left px-3 py-2 text-sm text-muted-foreground whitespace-nowrap"
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
                    {DataHeader ? (
                      <DataHeader stat={columnId} sortInfo={sortInfo} onSort={onSort} {...shared} />
                    ) : (
                      <th
                        className="text-right px-3 py-2 text-sm whitespace-nowrap"
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
                        {GroupCell && (
                          <GroupCell
                            row={row as TRow}
                            groupKey={row.groupKeys[col] as PivotTableGroupKeyEntry}
                            rowSpan={spans[col]!}
                            columnIndex={col}
                            {...shared}
                          />
                        )}
                      </React.Fragment>
                    ) : null
                  )}
                  {dataColumnIds.map(columnId => (
                    <React.Fragment key={columnId}>
                      {DataCell && <DataCell row={row as TRow} stat={columnId} {...shared} />}
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
