// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Search } from 'lucide-react';
import { useAllProfiles } from '@quent/client';
import type { ProfileRow } from '@quent/client';
import {
  Combobox,
  DataText,
  Input,
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@quent/components';
import type { ComboboxOption } from '@quent/components';
import { cn, formatDuration } from '@quent/utils';

const PAGE_SIZE_OPTIONS = [10, 25, 50, 100];
const DEFAULT_PAGE_SIZE = 25;

const NBSP_DASH = '—';

function engineName(row: ProfileRow): string {
  return row.engine.instance_name ?? row.engine.id;
}

function groupName(row: ProfileRow): string {
  return row.queryGroup?.instance_name ?? row.queryGroup?.id ?? '';
}

function queryName(row: ProfileRow): string {
  return row.query.instance_name ?? row.query.id;
}

/** Nanosecond epoch → locale date-time string. Coerces number|bigint runtimes. */
function formatStart(startUnixNs: bigint | null): string {
  if (startUnixNs == null) return NBSP_DASH;
  try {
    const ms = Number(BigInt(startUnixNs) / 1_000_000n);
    return Number.isFinite(ms) ? new Date(ms).toLocaleString() : NBSP_DASH;
  } catch {
    return NBSP_DASH;
  }
}

/** Phase offset in seconds → human duration, or dash when absent. */
function formatPhase(seconds: number | null): string {
  return seconds == null ? NBSP_DASH : formatDuration(seconds * 1000);
}

/** Compact page-number list with ellipsis sentinels (-1). */
function getPageItems(current: number, total: number): number[] {
  if (total <= 7) return Array.from({ length: total }, (_, i) => i);
  const items = new Set<number>([0, total - 1, current]);
  if (current > 0) items.add(current - 1);
  if (current < total - 1) items.add(current + 1);
  const sorted = [...items].sort((a, b) => a - b);
  const withGaps: number[] = [];
  let prev = -1;
  for (const page of sorted) {
    if (prev !== -1 && page - prev > 1) withGaps.push(-1);
    withGaps.push(page);
    prev = page;
  }
  return withGaps;
}

export function ProfileSearchPage() {
  const navigate = useNavigate();
  const { data, isLoading, isError, refetch } = useAllProfiles();

  const [text, setText] = useState('');
  const [engineId, setEngineId] = useState('');
  const [groupId, setGroupId] = useState('');
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);

  const rows = useMemo(() => data ?? [], [data]);

  const engineOptions = useMemo<ComboboxOption[]>(() => {
    const seen = new Map<string, string>();
    for (const row of rows) if (!seen.has(row.engine.id)) seen.set(row.engine.id, engineName(row));
    return [...seen].map(([value, label]) => ({ value, label }));
  }, [rows]);

  const groupOptions = useMemo<ComboboxOption[]>(() => {
    const seen = new Map<string, string>();
    for (const row of rows) {
      if (engineId && row.engine.id !== engineId) continue;
      const gid = row.queryGroup?.id;
      if (gid && !seen.has(gid)) seen.set(gid, groupName(row) || gid);
    }
    return [...seen].map(([value, label]) => ({ value, label }));
  }, [rows, engineId]);

  const filtered = useMemo(() => {
    const needle = text.trim().toLowerCase();
    return rows.filter(row => {
      if (engineId && row.engine.id !== engineId) return false;
      if (groupId && row.queryGroup?.id !== groupId) return false;
      if (needle) {
        const haystack = [
          row.query.id,
          row.query.instance_name,
          row.queryGroup?.id,
          row.queryGroup?.instance_name,
          row.engine.id,
          row.engine.instance_name,
        ]
          .filter(Boolean)
          .join(' ')
          .toLowerCase();
        if (!haystack.includes(needle)) return false;
      }
      return true;
    });
  }, [rows, text, engineId, groupId]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, totalPages - 1);
  const pageRows = filtered.slice(safePage * pageSize, safePage * pageSize + pageSize);

  const resetToFirstPage = () => setPage(0);

  const openProfile = (row: ProfileRow) => {
    navigate({
      to: '/profile/engine/$engineId/query/$queryId',
      params: { engineId: row.engine.id, queryId: row.query.id },
    });
  };

  const activeFilters = Boolean(text || engineId || groupId);
  const clearFilters = () => {
    setText('');
    setEngineId('');
    setGroupId('');
    resetToFirstPage();
  };

  return (
    <div className="mx-auto w-full max-w-7xl px-6 py-8 space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold">Search Profiles</h1>
        <p className="text-muted-foreground text-sm">
          Search and filter query profiles across all engines and query groups. Select a row to open
          its execution plan and timelines.
        </p>
      </header>

      {/* Filters */}
      <div className="space-y-3">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)_minmax(0,1fr)]">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              type="search"
              className="pl-9"
              placeholder="Search by query, group, or engine…"
              value={text}
              onChange={e => {
                setText(e.target.value);
                resetToFirstPage();
              }}
              aria-label="Search profiles"
            />
          </div>
          <Combobox
            options={engineOptions}
            value={engineId}
            onValueChange={value => {
              setEngineId(value);
              setGroupId('');
              resetToFirstPage();
            }}
            placeholder="All engines"
            searchPlaceholder="Filter engines…"
            emptyText="No engines"
            aria-label="Filter by engine"
          />
          <Combobox
            options={groupOptions}
            value={groupId}
            onValueChange={value => {
              setGroupId(value);
              resetToFirstPage();
            }}
            placeholder="All query groups"
            searchPlaceholder="Filter query groups…"
            emptyText="No query groups"
            aria-label="Filter by query group"
          />
        </div>

        {activeFilters && (
          <div className="flex items-center justify-end">
            <button
              type="button"
              onClick={clearFilters}
              className="text-xs text-primary hover:underline cursor-pointer"
            >
              Clear filters
            </button>
          </div>
        )}
      </div>

      {/* Results summary */}
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <span>
          {isLoading
            ? 'Loading profiles…'
            : `${filtered.length} ${filtered.length === 1 ? 'profile' : 'profiles'}${
                activeFilters ? ` (of ${rows.length})` : ''
              }`}
        </span>
        <div className="flex items-center gap-2">
          <span className="whitespace-nowrap">Rows per page</span>
          <Select
            value={String(pageSize)}
            onValueChange={value => {
              setPageSize(Number(value));
              resetToFirstPage();
            }}
          >
            <SelectTrigger className="h-8 w-20">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PAGE_SIZE_OPTIONS.map(size => (
                <SelectItem key={size} value={String(size)}>
                  {size}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Query</TableHead>
              <TableHead>Engine</TableHead>
              <TableHead>Query Group</TableHead>
              <TableHead>Started</TableHead>
              <TableHead className="text-right">Planning</TableHead>
              <TableHead className="text-right">Executing</TableHead>
              <TableHead className="text-right">Completed</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoading ? (
              Array.from({ length: 8 }).map((_, i) => (
                <TableRow key={i}>
                  {Array.from({ length: 7 }).map((__, j) => (
                    <TableCell key={j}>
                      <Skeleton className="h-4 w-full" />
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : isError ? (
              <TableRow>
                <TableCell colSpan={7} className="h-24 text-center text-muted-foreground">
                  Failed to load profiles.{' '}
                  <button
                    type="button"
                    onClick={() => refetch()}
                    className="text-primary hover:underline cursor-pointer"
                  >
                    Retry
                  </button>
                </TableCell>
              </TableRow>
            ) : pageRows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="h-24 text-center text-muted-foreground">
                  {rows.length === 0 ? 'No profiles available.' : 'No profiles match your filters.'}
                </TableCell>
              </TableRow>
            ) : (
              pageRows.map(row => (
                <TableRow
                  key={`${row.engine.id}:${row.query.id}`}
                  onClick={() => openProfile(row)}
                  className="cursor-pointer"
                >
                  <TableCell className="max-w-[22rem] truncate">
                    <DataText>{queryName(row)}</DataText>
                  </TableCell>
                  <TableCell>
                    <DataText>{engineName(row)}</DataText>
                  </TableCell>
                  <TableCell className="max-w-[16rem] truncate">
                    <DataText>{groupName(row) || NBSP_DASH}</DataText>
                  </TableCell>
                  <TableCell>
                    <DataText className="text-muted-foreground">
                      {formatStart(row.query.start_unix_ns)}
                    </DataText>
                  </TableCell>
                  <TableCell className="text-right">
                    <DataText>{formatPhase(row.query.planning_s)}</DataText>
                  </TableCell>
                  <TableCell className="text-right">
                    <DataText>{formatPhase(row.query.executing_s)}</DataText>
                  </TableCell>
                  <TableCell className="text-right">
                    <DataText>{formatPhase(row.query.completed_s)}</DataText>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      {!isLoading && !isError && totalPages > 1 && (
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                onClick={() => setPage(p => Math.max(0, p - 1))}
                disabled={safePage === 0}
                className={cn(safePage === 0 && 'pointer-events-none opacity-50')}
              />
            </PaginationItem>
            {getPageItems(safePage, totalPages).map((item, idx) =>
              item === -1 ? (
                <PaginationItem key={`gap-${idx}`}>
                  <PaginationEllipsis />
                </PaginationItem>
              ) : (
                <PaginationItem key={item}>
                  <PaginationLink isActive={item === safePage} onClick={() => setPage(item)}>
                    {item + 1}
                  </PaginationLink>
                </PaginationItem>
              )
            )}
            <PaginationItem>
              <PaginationNext
                onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))}
                disabled={safePage >= totalPages - 1}
                className={cn(safePage >= totalPages - 1 && 'pointer-events-none opacity-50')}
              />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}
    </div>
  );
}
