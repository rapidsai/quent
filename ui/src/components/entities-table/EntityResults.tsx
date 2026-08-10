// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { ChevronDown, ChevronFirst, ChevronLast, ChevronUp, ChevronsUpDown } from 'lucide-react';
import {
  Button,
  Input,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  thinScrollbarClass,
} from '@quent/components';
import { cn, formatDuration } from '@quent/utils';
import type { FiniteStateMachine } from '@quent/utils';
import type { EntityTableRow } from './types';

type SortColumn = 'instance' | 'type' | 'states' | 'start' | 'end' | 'span' | 'usage' | 'id';
type LocalSortDir = 'asc' | 'desc';

interface EntityResultsProps {
  rows: EntityTableRow[];
  selected: FiniteStateMachine | null;
  isError: boolean;
  isLoading: boolean;
  error: unknown;
  requestPending: boolean;
  hasValidationErrors: boolean;
  page: number;
  pageCount: number;
  paginationDisabled: boolean;
  total: number;
  visibleStart: number;
  visibleEnd: number;
  onSelect: (fsm: FiniteStateMachine) => void;
  onPageChange: (page: number) => void;
}

export function EntityResults({
  rows,
  selected,
  isError,
  isLoading,
  error,
  requestPending,
  hasValidationErrors,
  page,
  pageCount,
  paginationDisabled,
  total,
  visibleStart,
  visibleEnd,
  onSelect,
  onPageChange,
}: EntityResultsProps) {
  const [sortCol, setSortCol] = useState<SortColumn | null>(null);
  const [localSortDir, setLocalSortDir] = useState<LocalSortDir>('asc');

  function handleHeaderClick(col: SortColumn) {
    if (sortCol === col) {
      setLocalSortDir(d => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortCol(col);
      setLocalSortDir('asc');
    }
  }

  const sortedRows = useMemo(() => {
    if (!sortCol) return rows;
    return [...rows].sort((a, b) => {
      let cmp = 0;
      switch (sortCol) {
        case 'instance':
          cmp = a.fsm.instance_name.localeCompare(b.fsm.instance_name);
          break;
        case 'type':
          cmp = a.fsm.type_name.localeCompare(b.fsm.type_name);
          break;
        case 'states':
          cmp = a.fsm.transitions.length - b.fsm.transitions.length;
          break;
        case 'start':
          cmp = a.start - b.start;
          break;
        case 'end':
          cmp = a.end - b.end;
          break;
        case 'span':
          cmp = a.end - a.start - (b.end - b.start);
          break;
        case 'usage':
          cmp = a.usageDurationS - b.usageDurationS;
          break;
        case 'id':
          cmp = a.fsm.id.localeCompare(b.fsm.id);
          break;
      }
      return localSortDir === 'asc' ? cmp : -cmp;
    });
  }, [rows, sortCol, localSortDir]);

  const headProps = { sortCol, localSortDir, onSort: handleHeaderClick };

  return (
    <>
      <div
        aria-busy={requestPending}
        className={cn(
          'flex-1 min-h-0 overflow-auto transition-opacity duration-150',
          thinScrollbarClass,
          requestPending && rows.length > 0 ? 'opacity-60' : 'opacity-100'
        )}
      >
        {isError ? (
          <div className="p-4 text-sm text-destructive">
            Failed to load entities: {error instanceof Error ? error.message : 'unknown error'}
          </div>
        ) : (
          <Table containerClassName="relative w-full overflow-x-visible">
            <TableHeader className="sticky top-0 z-10 bg-card">
              <TableRow>
                <SortableHead col="instance" label="Instance" {...headProps} />
                <SortableHead col="type" label="Type" {...headProps} />
                <SortableHead col="states" label="States" className="text-right" {...headProps} />
                <SortableHead col="start" label="Start" className="text-right" {...headProps} />
                <SortableHead col="end" label="End" className="text-right" {...headProps} />
                <SortableHead col="span" label="FSM span" className="text-right" {...headProps} />
                <SortableHead
                  col="usage"
                  label="Longest usage"
                  className="text-right"
                  {...headProps}
                />
                <SortableHead col="id" label="ID" {...headProps} />
              </TableRow>
            </TableHeader>
            <TableBody>
              {sortedRows.map(row => (
                <EntityRow
                  key={row.fsm.id}
                  row={row}
                  selected={selected?.id === row.fsm.id}
                  onSelect={onSelect}
                />
              ))}
            </TableBody>
          </Table>
        )}
        {!isError && !isLoading && !hasValidationErrors && rows.length === 0 && (
          <div className="p-4 text-sm text-muted-foreground">No entities match the filters.</div>
        )}
        {isLoading && <div className="p-4 text-sm text-muted-foreground">Loading…</div>}
      </div>

      <div className="shrink-0 border-t bg-card p-2 flex items-center justify-between text-xs text-muted-foreground">
        <span>
          {visibleStart}–{visibleEnd} of {total} {total === 1 ? 'entity' : 'entities'}
        </span>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            aria-label="First page"
            disabled={paginationDisabled || page <= 0}
            onClick={() => onPageChange(0)}
          >
            <ChevronFirst className="size-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={paginationDisabled || page <= 0}
            onClick={() => onPageChange(Math.max(0, page - 1))}
          >
            Previous
          </Button>
          <PageJump
            page={page}
            pageCount={pageCount}
            disabled={paginationDisabled}
            onPageChange={onPageChange}
          />
          <Button
            variant="outline"
            size="sm"
            disabled={paginationDisabled || page + 1 >= pageCount}
            onClick={() => onPageChange(page + 1)}
          >
            Next
          </Button>
          <Button
            variant="outline"
            size="sm"
            aria-label="Last page"
            disabled={paginationDisabled || page + 1 >= pageCount}
            onClick={() => onPageChange(pageCount - 1)}
          >
            <ChevronLast className="size-4" />
          </Button>
        </div>
      </div>
    </>
  );
}

function SortableHead({
  col,
  label,
  className,
  sortCol,
  localSortDir,
  onSort,
}: {
  col: SortColumn;
  label: string;
  className?: string;
  sortCol: SortColumn | null;
  localSortDir: LocalSortDir;
  onSort: (col: SortColumn) => void;
}) {
  const isActive = sortCol === col;
  const isRight = className?.includes('text-right');
  return (
    <TableHead
      className={cn('cursor-pointer select-none', className)}
      onClick={() => onSort(col)}
    >
      <span className={cn('flex items-center gap-1', isRight && 'justify-end')}>
        {label}
        {isActive ? (
          localSortDir === 'asc' ? (
            <ChevronUp className="size-3.5 shrink-0" />
          ) : (
            <ChevronDown className="size-3.5 shrink-0" />
          )
        ) : (
          <ChevronsUpDown className="size-3.5 shrink-0 opacity-30" />
        )}
      </span>
    </TableHead>
  );
}

function PageJump({
  page,
  pageCount,
  disabled,
  onPageChange,
}: {
  page: number;
  pageCount: number;
  disabled: boolean;
  onPageChange: (page: number) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const displayValue = draft ?? String(page + 1);

  function commit() {
    if (draft === null) return;
    const parsed = parseInt(draft, 10);
    if (Number.isFinite(parsed)) {
      onPageChange(Math.min(pageCount - 1, Math.max(0, parsed - 1)));
    }
    setDraft(null);
  }

  return (
    <span className="flex items-center gap-1.5">
      Page
      <Input
        type="number"
        min={1}
        max={pageCount}
        aria-label="Page number"
        disabled={disabled}
        className="h-7 w-14 px-1.5 text-center tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
        value={displayValue}
        onChange={e => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={e => {
          if (e.key === 'Enter') {
            commit();
            e.currentTarget.blur();
          } else if (e.key === 'Escape') {
            setDraft(null);
            e.currentTarget.blur();
          }
        }}
      />
      / {pageCount}
    </span>
  );
}

function EntityRow({
  row,
  selected,
  onSelect,
}: {
  row: EntityTableRow;
  selected: boolean;
  onSelect: (fsm: FiniteStateMachine) => void;
}) {
  const select = () => onSelect(row.fsm);

  return (
    <TableRow
      className="cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
      tabIndex={0}
      aria-selected={selected}
      data-state={selected ? 'selected' : undefined}
      onClick={select}
      onKeyDown={event => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          select();
        } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault();
          const rows = Array.from(
            event.currentTarget.closest('tbody')?.querySelectorAll<HTMLElement>('tr') ?? []
          );
          const currentIndex = rows.indexOf(event.currentTarget);
          const next = rows[currentIndex + (event.key === 'ArrowDown' ? 1 : -1)];
          next?.focus();
        }
      }}
    >
      <TableCell className="font-medium">{row.fsm.instance_name}</TableCell>
      <TableCell>{row.fsm.type_name}</TableCell>
      <TableCell className="text-right tabular-nums">{row.fsm.transitions.length}</TableCell>
      <TableCell className="text-right tabular-nums">{row.start.toFixed(3)}s</TableCell>
      <TableCell className="text-right tabular-nums">{row.end.toFixed(3)}s</TableCell>
      <TableCell className="text-right tabular-nums">
        {formatDuration((row.end - row.start) * 1000)}
      </TableCell>
      <TableCell className="text-right tabular-nums">
        {formatDuration(row.usageDurationS * 1000)}
      </TableCell>
      <TableCell className="font-mono text-xs text-muted-foreground">{row.fsm.id}</TableCell>
    </TableRow>
  );
}
