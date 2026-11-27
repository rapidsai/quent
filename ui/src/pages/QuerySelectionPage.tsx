// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Link, useNavigate } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { ArrowLeft, ListX } from 'lucide-react';
import { queriesQueryOptions, useEngines } from '@quent/client';
import { Button, DataText } from '@quent/components';
import { formatDuration } from '@quent/utils';
import type { Query } from '@quent/utils';
import {
  EntityCatalogTable,
  type CatalogMetadataColumn,
} from '@/components/query-selection/EntityCatalogTable';
import { sortQueriesByMostRecent } from '@/components/query-selection/catalog';

const queryColumns: CatalogMetadataColumn<Query>[] = [
  {
    id: 'started',
    label: 'Started',
    render: query => (
      <DataText className="tabular-nums">
        {query.start_unix_ns == null
          ? '—'
          : new Date(Number(query.start_unix_ns) / 1_000_000).toLocaleString()}
      </DataText>
    ),
  },
  {
    id: 'planning',
    label: 'Planning',
    render: query => (
      <DataText className="tabular-nums">
        {query.planning_s == null ? '—' : formatDuration(query.planning_s * 1000)}
      </DataText>
    ),
  },
  {
    id: 'duration',
    label: 'Duration',
    render: query => (
      <DataText className="tabular-nums">
        {query.completed_s == null ? '—' : formatDuration(query.completed_s * 1000)}
      </DataText>
    ),
  },
];

export function QuerySelectionPage({ engineId }: { engineId: string }) {
  const navigate = useNavigate();
  const enginesList = useEngines();
  const queryList = useQuery(queriesQueryOptions(engineId));
  const selectedEngine = enginesList.data?.items.find(engine => engine.id === engineId);
  const queries = useMemo(
    () => sortQueriesByMostRecent(queryList.data?.items ?? []),
    [queryList.data?.items]
  );

  return (
    <div className="mx-auto flex h-full min-h-0 w-full max-w-7xl flex-col px-4 py-8 sm:px-6 lg:px-8">
      <div className="mb-6 flex shrink-0 items-center gap-3 border-b border-border pb-5">
        <Button
          variant="ghost"
          size="icon"
          aria-label="Change engine"
          className="size-12 shrink-0 [&_svg]:size-7"
          onClick={() => navigate({ to: '/' })}
        >
          <ArrowLeft />
        </Button>
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Selected engine
          </p>
          <DataText as="p" className="mt-1 text-base font-semibold">
            {selectedEngine?.instance_name ?? engineId}
          </DataText>
        </div>
      </div>

      <EntityCatalogTable
        title="Select a query"
        description="Search query metadata or use the analyzer’s suggested grouping to find the profile you want to inspect."
        items={queries}
        initialGroupBy={queryList.data?.initial_group_by_attribute ?? null}
        metadataColumns={queryColumns}
        isLoading={queryList.isLoading}
        error={queryList.error instanceof Error ? queryList.error : null}
        emptyMessage="This engine has no queries."
        emptyIcon={ListX}
        onRetry={() => void queryList.refetch()}
        renderItemLink={(query, children, primary) => (
          <Link
            to="/profile/engine/$engineId/query/$queryId"
            params={{ engineId, queryId: query.id }}
            search={{}}
            aria-label={primary ? `View profile: ${query.instance_name ?? query.id}` : undefined}
            tabIndex={primary ? undefined : -1}
          >
            {children}
          </Link>
        )}
      />
    </div>
  );
}
