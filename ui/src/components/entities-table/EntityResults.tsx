// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState, type KeyboardEvent, type MouseEvent } from 'react';
import { ChevronDown, ChevronFirst, ChevronLast, ChevronUp, LoaderCircle } from 'lucide-react';
import {
  DataText,
  Input,
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  thinScrollbarClass,
} from '@quent/components';
import { cn, formatDuration } from '@quent/utils';
import type { FiniteStateMachine, SortDir } from '@quent/utils';
import type { EntityTableRow } from './types';
import { MAX_PAGE_SIZE, SORT_ASC, SORT_DESC, normalizePageSize } from './utils';

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
  pageSize: number | null;
  paginationDisabled: boolean;
  total: number;
  visibleStart: number;
  visibleEnd: number;
  sortDir: SortDir;
  stateColorFn: (name: string) => string;
  onSelect: (fsm: FiniteStateMachine) => void;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number | null) => void;
  onSortChange: (dir: SortDir) => void;
}

/**
 * `PaginationLink`/`PaginationPrevious`/`PaginationNext` render bare anchors
 * (no `href`), which aren't keyboard-operable by default — this restores
 * Enter/Space activation and standard disabled affordances.
 */
function navLinkProps(disabled: boolean, onClick: () => void) {
  return {
    'aria-disabled': disabled,
    tabIndex: disabled ? -1 : 0,
    className: cn(disabled && 'pointer-events-none opacity-50'),
    onClick: (event: MouseEvent) => {
      event.preventDefault();
      if (!disabled) onClick();
    },
    onKeyDown: (event: KeyboardEvent) => {
      if (disabled) return;
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onClick();
      }
    },
  };
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
  pageSize,
  paginationDisabled,
  total,
  visibleStart,
  visibleEnd,
  sortDir,
  stateColorFn,
  onSelect,
  onPageChange,
  onPageSizeChange,
  onSortChange,
}: EntityResultsProps) {
  function handleUsageHeaderClick() {
    onSortChange(sortDir === SORT_ASC ? SORT_DESC : SORT_ASC);
  }

  const firstPageDisabled = paginationDisabled || page <= 0;
  const lastPageDisabled = paginationDisabled || page + 1 >= pageCount;

  return (
    <>
      <div className="relative flex-1 min-h-0">
        <div aria-busy={requestPending} className={cn('h-full overflow-auto', thinScrollbarClass)}>
          {isError ? (
            <div className="p-4 text-sm text-destructive">
              Failed to load entities: {error instanceof Error ? error.message : 'unknown error'}
            </div>
          ) : (
            <Table containerClassName="relative w-full overflow-x-visible">
              <TableHeader className="sticky top-0 z-10 bg-card">
                <TableRow>
                  <TableHead>Instance</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead className="text-right">States</TableHead>
                  <TableHead>Sequence</TableHead>
                  <TableHead className="text-right">Start</TableHead>
                  <TableHead className="text-right">End</TableHead>
                  <TableHead className="text-right">FSM span</TableHead>
                  <SortableHead
                    label="Longest usage"
                    className="text-right"
                    sortDir={sortDir}
                    onSort={handleUsageHeaderClick}
                  />
                  <TableHead>ID</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map(row => (
                  <EntityRow
                    key={row.fsm.id}
                    row={row}
                    selected={selected?.id === row.fsm.id}
                    stateColorFn={stateColorFn}
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

        {requestPending && rows.length > 0 && (
          <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-background/70 backdrop-blur-[1px]">
            <span
              role="status"
              aria-live="polite"
              className="flex items-center gap-2 rounded-lg border bg-card px-4 py-2.5 text-sm font-medium text-foreground shadow-lg"
            >
              <LoaderCircle className="size-4 animate-spin text-primary" />
              Updating…
            </span>
          </div>
        )}
      </div>

      <div className="shrink-0 border-t bg-card p-2 flex items-center justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-3">
          <span>
            {visibleStart}–{visibleEnd} of {total} {total === 1 ? 'entity' : 'entities'}
          </span>
          <PageSizeField value={pageSize} onChange={onPageSizeChange} />
        </div>
        <Pagination className="mx-0 w-auto">
          <PaginationContent className="gap-1">
            <PaginationItem>
              <PaginationLink
                aria-label="First page"
                size="icon"
                {...navLinkProps(firstPageDisabled, () => onPageChange(0))}
              >
                <ChevronFirst className="size-4" />
              </PaginationLink>
            </PaginationItem>
            <PaginationItem>
              <PaginationPrevious
                {...navLinkProps(firstPageDisabled, () => onPageChange(Math.max(0, page - 1)))}
              />
            </PaginationItem>
            <PaginationItem className="mx-1.5">
              <PageJump
                page={page}
                pageCount={pageCount}
                disabled={paginationDisabled}
                onPageChange={onPageChange}
              />
            </PaginationItem>
            <PaginationItem>
              <PaginationNext {...navLinkProps(lastPageDisabled, () => onPageChange(page + 1))} />
            </PaginationItem>
            <PaginationItem>
              <PaginationLink
                aria-label="Last page"
                size="icon"
                {...navLinkProps(lastPageDisabled, () => onPageChange(pageCount - 1))}
              >
                <ChevronLast className="size-4" />
              </PaginationLink>
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      </div>
    </>
  );
}

function SortableHead({
  label,
  className,
  sortDir,
  onSort,
}: {
  label: string;
  className?: string;
  sortDir: SortDir;
  onSort: () => void;
}) {
  const isRight = className?.includes('text-right');
  return (
    <TableHead
      aria-sort={sortDir === SORT_ASC ? 'ascending' : 'descending'}
      className={cn('p-0 select-none', className)}
    >
      <button
        type="button"
        className={cn(
          'flex h-10 w-full cursor-pointer items-center gap-1 px-2',
          isRight && 'justify-end'
        )}
        onClick={onSort}
      >
        {label}
        {sortDir === SORT_ASC ? (
          <ChevronUp className="size-3.5 shrink-0" />
        ) : (
          <ChevronDown className="size-3.5 shrink-0" />
        )}
      </button>
    </TableHead>
  );
}

function PageSizeField({
  value,
  onChange,
}: {
  value: number | null;
  onChange: (value: number | null) => void;
}) {
  return (
    <label className="flex items-center gap-1.5">
      Page size
      <Input
        type="number"
        min={1}
        max={MAX_PAGE_SIZE}
        step={1}
        aria-label="Page size"
        className="h-7 w-16 px-1.5 text-center tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
        value={value ?? ''}
        onChange={event => onChange(event.target.value === '' ? null : event.target.valueAsNumber)}
        onBlur={() => onChange(normalizePageSize(value))}
      />
    </label>
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
  stateColorFn,
  onSelect,
}: {
  row: EntityTableRow;
  selected: boolean;
  stateColorFn: (name: string) => string;
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
      <TableCell className="font-medium">
        <DataText>{row.fsm.instance_name}</DataText>
      </TableCell>
      <TableCell>
        <DataText>{row.fsm.type_name}</DataText>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <DataText>{row.fsm.transitions.length}</DataText>
      </TableCell>
      <TableCell>
        <div className="flex h-3.5 w-24 gap-px overflow-hidden rounded-sm">
          {row.fsm.transitions.map((t, i) => (
            <div key={i} className="flex-1" style={{ backgroundColor: stateColorFn(t.name) }} />
          ))}
        </div>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <DataText>{row.start.toFixed(3)}s</DataText>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <DataText>{row.end.toFixed(3)}s</DataText>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <DataText>{formatDuration((row.end - row.start) * 1000)}</DataText>
      </TableCell>
      <TableCell className="text-right tabular-nums">
        <DataText>{formatDuration(row.usageDurationS * 1000)}</DataText>
      </TableCell>
      <TableCell className="text-xs text-muted-foreground">
        <DataText>{row.fsm.id}</DataText>
      </TableCell>
    </TableRow>
  );
}
