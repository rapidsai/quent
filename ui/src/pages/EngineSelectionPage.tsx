// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Link } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { Server } from 'lucide-react';
import { fetchListEngines } from '@quent/client';
import { DataText } from '@quent/components';
import type { Engine } from '@quent/utils';
import {
  EntityCatalogTable,
  type CatalogMetadataColumn,
} from '@/components/query-selection/EntityCatalogTable';

const engineColumns: CatalogMetadataColumn<Engine>[] = [
  {
    id: 'implementation',
    label: 'Implementation',
    render: engine => (
      <DataText>
        {[engine.implementation?.name, engine.implementation?.version].filter(Boolean).join(' ') ||
          '—'}
      </DataText>
    ),
  },
];

export function EngineSelectionPage() {
  const enginesList = useQuery({
    queryKey: ['list_engines'],
    queryFn: fetchListEngines,
  });

  return (
    <div className="mx-auto flex h-full min-h-0 w-full max-w-7xl flex-col px-4 py-8 sm:px-6 lg:px-8">
      <EntityCatalogTable
        title="Select an engine"
        description="Search recorded engines and compare their runtime metadata before choosing a query."
        items={enginesList.data?.items ?? []}
        initialGroupBy={enginesList.data?.initial_group_by_attribute ?? null}
        metadataColumns={engineColumns}
        isLoading={enginesList.isLoading}
        error={enginesList.error instanceof Error ? enginesList.error : null}
        emptyMessage="No engines are available."
        emptyIcon={Server}
        onRetry={() => void enginesList.refetch()}
        renderItemLink={(engine, children, primary) => (
          <Link
            to="/profile/engine/$engineId"
            params={{ engineId: engine.id }}
            aria-label={primary ? `View queries: ${engine.instance_name ?? engine.id}` : undefined}
            tabIndex={primary ? undefined : -1}
          >
            {children}
          </Link>
        )}
      />
    </div>
  );
}
