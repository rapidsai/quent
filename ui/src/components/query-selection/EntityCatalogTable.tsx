// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { RotateCcw, Search, type LucideIcon } from 'lucide-react';
import {
  Button,
  DataText,
  EmptyState,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Skeleton,
  InnerScrollTable,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@quent/components';
import { formatAttributeValue } from '@quent/utils';
import {
  attributeLabel,
  attributeValue,
  catalogAttributeKeys,
  filterCatalogItems,
  groupCatalogItems,
  type CatalogEntity,
} from './catalog';

const NO_GROUPING = '__none__';
const LOADING_DELAY_MS = 200;
const linkedCellClassName =
  'p-0 [&>a]:block [&>a]:px-2 [&>a]:py-2 [&>a]:focus-visible:outline-none [&>a]:focus-visible:ring-1 [&>a]:focus-visible:ring-inset [&>a]:focus-visible:ring-ring';

export interface CatalogMetadataColumn<T> {
  id: string;
  label: string;
  render: (item: T) => ReactNode;
}

export interface EntityCatalogTableProps<T extends CatalogEntity> {
  title: string;
  description: string;
  items: T[];
  initialGroupBy: string | null;
  metadataColumns?: CatalogMetadataColumn<T>[];
  isLoading: boolean;
  error: Error | null;
  emptyMessage: string;
  /** Icon for the "nothing to list" state; the search no-match state keeps its own. */
  emptyIcon?: LucideIcon;
  onRetry: () => void;
  renderItemLink: (item: T, children: ReactNode, primary: boolean) => ReactNode;
}

export function EntityCatalogTable<T extends CatalogEntity>({
  title,
  description,
  items,
  initialGroupBy,
  metadataColumns = [],
  isLoading,
  error,
  emptyMessage,
  emptyIcon,
  onRetry,
  renderItemLink,
}: EntityCatalogTableProps<T>) {
  const [search, setSearch] = useState('');
  const [groupByOverride, setGroupByOverride] = useState<string>();
  const attributeKeys = useMemo(() => catalogAttributeKeys(items), [items]);
  const hintedGroupBy =
    initialGroupBy && attributeKeys.includes(initialGroupBy) ? initialGroupBy : NO_GROUPING;
  const selectedGroupBy = groupByOverride ?? hintedGroupBy;
  const groupBy = selectedGroupBy === NO_GROUPING ? '' : selectedGroupBy;
  const filteredItems = useMemo(() => filterCatalogItems(items, search), [items, search]);
  const groups = useMemo(() => groupCatalogItems(filteredItems, groupBy), [filteredItems, groupBy]);
  const columnCount = 1 + metadataColumns.length + attributeKeys.length;
  const catalogName = title.replace(/^Select (?:an?|the) /i, '');

  return (
    <section aria-labelledby="catalog-title" className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="shrink-0">
        <h1 id="catalog-title" className="text-2xl font-semibold tracking-tight text-balance">
          {title}
        </h1>
        <p className="mt-1 max-w-2xl text-sm text-muted-foreground text-pretty">{description}</p>
      </div>

      <div className="flex shrink-0 flex-col gap-2 sm:flex-row sm:items-center">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={search}
            onChange={event => setSearch(event.target.value)}
            placeholder={`Search ${catalogName}…`}
            aria-label={`Search ${catalogName}`}
            className="pl-9"
          />
        </div>
        <div className="flex items-center gap-2">
          <label htmlFor="catalog-group-by" className="text-sm text-muted-foreground">
            Group by
          </label>
          <Select value={selectedGroupBy} onValueChange={setGroupByOverride}>
            <SelectTrigger id="catalog-group-by" className="w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={NO_GROUPING}>None</SelectItem>
              {attributeKeys.map(key => (
                <SelectItem key={key} value={key}>
                  {attributeLabel(key)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <div className="flex flex-col overflow-hidden rounded-md border border-border bg-card">
        {isLoading ? (
          <CatalogLoading />
        ) : error ? (
          <CatalogError error={error} onRetry={onRetry} />
        ) : items.length === 0 ? (
          <CatalogMessage icon={emptyIcon}>{emptyMessage}</CatalogMessage>
        ) : filteredItems.length === 0 ? (
          <CatalogMessage>No matches for “{search}”.</CatalogMessage>
        ) : (
          <InnerScrollTable stickyLeftColumns={1} containerClassName="min-h-0 flex-auto">
            <TableHeader className="bg-card">
              <TableRow>
                <TableHead className="min-w-48">Name</TableHead>
                {metadataColumns.map(column => (
                  <TableHead key={column.id}>{column.label}</TableHead>
                ))}
                {attributeKeys.map(key => (
                  <TableHead key={key}>{attributeLabel(key)}</TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {groups.flatMap(group => [
                ...(groupBy
                  ? [
                      <TableRow key={`group-${group.id}`} className="hover:bg-muted/30">
                        <TableCell
                          colSpan={columnCount}
                          className="border-b border-border bg-muted py-2 text-xs font-semibold text-muted-foreground"
                        >
                          <div className="sticky left-2 w-fit">
                            {attributeLabel(groupBy)}: {group.label}
                            <span className="ml-2 font-normal tabular-nums">
                              ({group.items.length})
                            </span>
                          </div>
                        </TableCell>
                      </TableRow>,
                    ]
                  : []),
                ...group.items.map(item => (
                  <TableRow key={item.id} className="cursor-pointer">
                    <TableCell className={`${linkedCellClassName} min-w-48 max-w-64 font-medium`}>
                      {renderItemLink(
                        item,
                        <DataText className="block truncate">
                          {item.instance_name ?? 'Unnamed'}
                        </DataText>,
                        true
                      )}
                    </TableCell>
                    {metadataColumns.map(column => (
                      <TableCell key={column.id} className={linkedCellClassName}>
                        {renderItemLink(item, column.render(item), false)}
                      </TableCell>
                    ))}
                    {attributeKeys.map(key => (
                      <TableCell key={key} className={`${linkedCellClassName} max-w-64`}>
                        {renderItemLink(
                          item,
                          <DataText className="block truncate">
                            {formatAttributeValue(key, attributeValue(item.custom_attributes, key))}
                          </DataText>,
                          false
                        )}
                      </TableCell>
                    ))}
                  </TableRow>
                )),
              ])}
            </TableBody>
          </InnerScrollTable>
        )}
      </div>
    </section>
  );
}

function CatalogLoading() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const timeout = window.setTimeout(() => setVisible(true), LOADING_DELAY_MS);
    return () => window.clearTimeout(timeout);
  }, []);

  if (!visible) {
    return null;
  }

  return (
    <div aria-label="Loading catalog" className="flex-1 space-y-3 p-4">
      {Array.from({ length: 5 }, (_, index) => (
        <Skeleton key={index} className="h-9 w-full" />
      ))}
    </div>
  );
}

function CatalogError({ error, onRetry }: { error: Error; onRetry: () => void }) {
  return (
    <div role="alert" className="flex flex-1 flex-col items-center justify-center gap-3 p-6">
      <div className="text-center">
        <p className="font-medium">Unable to load this catalog</p>
        <p className="mt-1 text-sm text-muted-foreground">{error.message}</p>
      </div>
      <Button variant="outline" size="sm" onClick={onRetry}>
        <RotateCcw />
        Retry
      </Button>
    </div>
  );
}

function CatalogMessage({ children, icon }: { children: ReactNode; icon?: LucideIcon }) {
  return <EmptyState title={children} icon={icon} className="flex-1" />;
}
