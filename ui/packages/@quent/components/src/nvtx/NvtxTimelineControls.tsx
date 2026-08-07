// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { BarChart3, ListFilter } from 'lucide-react';
import type { NvtxCatalog, NvtxDomainSelection, NvtxRangeStatistics } from '@quent/utils';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import { formatNvtxDuration } from './NvtxLaneChart.utils';

export interface NvtxCatalogControlEntry {
  contextId: string;
  label: string;
  catalog: NvtxCatalog;
}

export interface NvtxStatisticsControlEntry {
  contextId: string;
  label: string;
  statistics: NvtxRangeStatistics[];
}

export interface NvtxTimelineControlsProps {
  catalogs: NvtxCatalogControlEntry[];
  selections: Readonly<Record<string, NvtxDomainSelection[]>>;
  statistics: NvtxStatisticsControlEntry[];
  onSelectionChange: (contextId: string, selections: NvtxDomainSelection[]) => void;
}

export function NvtxTimelineControls({
  catalogs,
  selections,
  statistics,
  onSelectionChange,
}: NvtxTimelineControlsProps) {
  const selectedCount = Object.values(selections).reduce(
    (total, domains) =>
      total +
      domains.reduce(
        (domainTotal, domain) =>
          domainTotal + domain.category_ids.length + (domain.include_uncategorized ? 1 : 0),
        0
      ),
    0
  );

  const toggleDomain = (entry: NvtxCatalogControlEntry, domainId: string) => {
    const current = selections[entry.contextId] ?? [];
    const selectionDomainId = domainId;
    const existing = current.find(selection => selection.domain_id === selectionDomainId);
    if (existing) {
      onSelectionChange(
        entry.contextId,
        current.filter(selection => selection.domain_id !== selectionDomainId)
      );
      return;
    }
    const domain = entry.catalog.domains.find(candidate => candidate.domain_id === domainId)!;
    onSelectionChange(entry.contextId, [
      ...current,
      {
        domain_id: selectionDomainId,
        category_ids: domain.categories.map(category => category.category_id),
        include_uncategorized: domain.has_uncategorized,
      },
    ]);
  };

  const toggleCategory = (
    entry: NvtxCatalogControlEntry,
    domainId: string,
    categoryId: number | null
  ) => {
    const current = selections[entry.contextId] ?? [];
    const selectionDomainId = domainId;
    const existing = current.find(selection => selection.domain_id === selectionDomainId);
    const categoryIds = new Set(existing?.category_ids ?? []);
    let includeUncategorized = existing?.include_uncategorized ?? false;
    if (categoryId === null) includeUncategorized = !includeUncategorized;
    else if (categoryIds.has(categoryId)) categoryIds.delete(categoryId);
    else categoryIds.add(categoryId);
    const replacement: NvtxDomainSelection = {
      domain_id: selectionDomainId,
      category_ids: [...categoryIds],
      include_uncategorized: includeUncategorized,
    };
    const withoutDomain = current.filter(selection => selection.domain_id !== selectionDomainId);
    onSelectionChange(
      entry.contextId,
      replacement.category_ids.length > 0 || replacement.include_uncategorized
        ? [...withoutDomain, replacement]
        : withoutDomain
    );
  };

  return (
    <>
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            className="inline-flex min-h-8 items-center gap-1 rounded-sm px-1.5 hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-primary"
            aria-label={`Filter NVTX lanes, ${selectedCount} options selected`}
          >
            <ListFilter className="h-3.5 w-3.5" />
            <span>Filter NVTX lanes</span>
            <span className="rounded bg-primary/15 px-1 text-primary">{selectedCount}</span>
          </button>
        </PopoverTrigger>
        <PopoverContent className="w-80 max-h-96 overflow-y-auto p-4 text-sm" align="end">
          {catalogs.map(entry => (
            <fieldset key={entry.contextId} className="mb-4 last:mb-0">
              {catalogs.length > 1 && (
                <legend
                  className="mb-2 max-w-full truncate text-xs font-semibold"
                  title={entry.label}
                >
                  {entry.label}
                </legend>
              )}
              {entry.catalog.domains
                .filter(domain => domain.categories.length > 0 || domain.has_uncategorized)
                .map(domain => {
                  const selection = (selections[entry.contextId] ?? []).find(
                    candidate => candidate.domain_id === domain.domain_id
                  );
                  return (
                    <details key={domain.domain_id.toString()} className="mb-2 last:mb-0">
                      <summary className="flex min-h-8 cursor-pointer list-none items-center gap-2">
                        <input
                          type="checkbox"
                          checked={selection !== undefined}
                          onClick={event => event.stopPropagation()}
                          onChange={() => toggleDomain(entry, domain.domain_id)}
                          aria-label={`${domain.name} domain`}
                          className="h-3.5 w-3.5 accent-primary"
                        />
                        <span
                          className="h-2.5 w-2.5 rounded-full shrink-0"
                          style={{ backgroundColor: domain.color }}
                        />
                        <span className="truncate" title={domain.name}>
                          {domain.name}
                        </span>
                      </summary>
                      <div className="ml-6 border-l border-border pl-2">
                        {domain.categories.map(category => (
                          <label
                            key={category.category_id}
                            className="flex min-h-8 items-center gap-2 cursor-pointer"
                          >
                            <input
                              type="checkbox"
                              checked={
                                selection?.category_ids.includes(category.category_id) ?? false
                              }
                              onChange={() =>
                                toggleCategory(entry, domain.domain_id, category.category_id)
                              }
                              className="h-3.5 w-3.5 accent-primary"
                            />
                            <span className="truncate" title={category.name}>
                              {category.name}
                            </span>
                          </label>
                        ))}
                        {domain.has_uncategorized && (
                          <label className="flex min-h-8 items-center gap-2 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={selection?.include_uncategorized ?? false}
                              onChange={() => toggleCategory(entry, domain.domain_id, null)}
                              className="h-3.5 w-3.5 accent-primary"
                            />
                            <span>Uncategorized</span>
                          </label>
                        )}
                      </div>
                    </details>
                  );
                })}
            </fieldset>
          ))}
        </PopoverContent>
      </Popover>

      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            className="inline-flex min-h-8 items-center gap-1 rounded-sm px-1.5 hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-primary"
          >
            <BarChart3 className="h-3.5 w-3.5" />
            <span>NVTX statistics</span>
          </button>
        </PopoverTrigger>
        <PopoverContent className="w-96 max-h-96 overflow-y-auto p-4" align="end">
          {statistics.every(entry => entry.statistics.length === 0) ? (
            <p className="text-sm text-muted-foreground">No NVTX ranges in this view</p>
          ) : (
            statistics.map(entry => (
              <section key={entry.contextId} className="mb-4 last:mb-0">
                {statistics.length > 1 && (
                  <h3 className="mb-2 truncate text-base font-semibold" title={entry.label}>
                    {entry.label}
                  </h3>
                )}
                <div className="space-y-2">
                  {entry.statistics.map((item, index) => (
                    <div
                      key={`${item.domain_id.toString()}-${item.category_id ?? 'none'}-${item.message}-${index}`}
                      className="rounded bg-secondary/40 p-2 text-xs"
                    >
                      <div className="font-semibold break-words text-sm">{item.message}</div>
                      <div className="text-muted-foreground break-words">
                        {item.domain_name} · {item.category_name ?? 'Uncategorized'}
                      </div>
                      <dl className="mt-1 grid grid-cols-2 gap-x-2 gap-y-1">
                        <dt>ranges</dt>
                        <dd className="text-right font-mono">{item.count.toString()}</dd>
                        <dt>observed</dt>
                        <dd className="text-right font-mono">{item.observed_count.toString()}</dd>
                        <dt>total visible</dt>
                        <dd className="text-right font-mono">
                          {formatNvtxDuration(item.total_duration)}
                        </dd>
                        <dt>average</dt>
                        <dd className="text-right font-mono">
                          {item.observed_count === 0n ? '—' : formatNvtxDuration(item.avg_duration)}
                        </dd>
                        <dt>minimum</dt>
                        <dd className="text-right font-mono">
                          {item.min_duration === null ? '—' : formatNvtxDuration(item.min_duration)}
                        </dd>
                        <dt>maximum</dt>
                        <dd className="text-right font-mono">
                          {item.max_duration === null ? '—' : formatNvtxDuration(item.max_duration)}
                        </dd>
                      </dl>
                      {item.observed_count !== item.count && (
                        <p className="mt-1 text-muted-foreground">
                          Includes incomplete observations; duration metrics cover closed ranges
                          only.
                        </p>
                      )}
                      {item.saturated && (
                        <p className="mt-1 text-muted-foreground">
                          Duration totals reached the maximum representable value.
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              </section>
            ))
          )}
        </PopoverContent>
      </Popover>
    </>
  );
}
