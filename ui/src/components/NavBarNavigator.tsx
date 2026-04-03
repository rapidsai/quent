// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMatch, useNavigate } from '@tanstack/react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ChevronDown, ChevronRight } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { queryBundleQueryOptions } from '@/hooks/useQueryBundle';
import { fetchListEngines, fetchListCoordinators, fetchListQueries } from '@/services/api';
import { DataText } from '@/components/ui/data-text';

function BreadcrumbDropdown({
  label,
  activeId,
  items,
  onSelect,
}: {
  label: string;
  activeId: string;
  items: { id: string; label: string }[] | undefined;
  onSelect: (id: string) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button className="flex items-center gap-0.5 px-1.5 py-0.5 -mx-1.5 rounded-sm hover:text-foreground hover:bg-accent transition-colors cursor-pointer">
          <DataText>{label}</DataText>
          <ChevronDown className="h-3 w-3 opacity-50" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-64 overflow-y-auto">
        {items?.map(item => (
          <DropdownMenuItem
            key={item.id}
            onSelect={() => onSelect(item.id)}
            className={cn(item.id === activeId && 'font-semibold bg-accent')}
          >
            <DataText>{item.label}</DataText>
          </DropdownMenuItem>
        ))}
        {(!items || items.length === 0) && <DropdownMenuItem disabled>No items</DropdownMenuItem>}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function NavBarNavigator() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const queryIndexMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/',
    shouldThrow: false,
  });
  const queryNodeMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/node/$nodeId',
    shouldThrow: false,
  });

  const engineId = queryIndexMatch?.params?.engineId ?? queryNodeMatch?.params?.engineId;
  const queryId = queryIndexMatch?.params?.queryId ?? queryNodeMatch?.params?.queryId;

  const { data: queryBundle } = useQuery({
    ...queryBundleQueryOptions({ engineId: engineId ?? '', queryId: queryId ?? '' }),
    enabled: !!engineId && !!queryId,
  });

  const queryGroupId = queryBundle?.entities.query_group.id;

  const { data: engines } = useQuery({
    queryKey: ['list_engines'],
    queryFn: fetchListEngines,
    enabled: !!engineId,
  });

  const { data: queryGroups } = useQuery({
    queryKey: ['list_coordinators', engineId],
    queryFn: () => fetchListCoordinators(engineId!),
    enabled: !!engineId,
  });

  const { data: queries } = useQuery({
    queryKey: ['list_queries', engineId, queryGroupId],
    queryFn: () => fetchListQueries(engineId!, queryGroupId!),
    enabled: !!engineId && !!queryGroupId,
  });

  if (!queryBundle || !engineId) return null;

  const engine = queryBundle.entities.engine.instance_name ?? queryBundle.entities.engine.id;
  const queryGroupName = queryBundle.entities.query_group.instance_name;
  const queryName = queryBundle.entities.query.instance_name;

  const handleEngineChange = async (newEngineId: string) => {
    if (newEngineId === engineId) return;
    try {
      const groups = await queryClient.fetchQuery({
        queryKey: ['list_coordinators', newEngineId],
        queryFn: () => fetchListCoordinators(newEngineId),
      });
      const firstGroup = groups[0];
      if (!firstGroup) return;
      const groupQueries = await queryClient.fetchQuery({
        queryKey: ['list_queries', newEngineId, firstGroup.id],
        queryFn: () => fetchListQueries(newEngineId, firstGroup.id),
      });
      const firstQuery = groupQueries[0];
      if (firstQuery) {
        navigate({
          to: '/profile/engine/$engineId/query/$queryId',
          params: { engineId: newEngineId, queryId: firstQuery.id },
        });
      }
    } catch {
      // ignore
    }
  };

  const handleQueryGroupChange = async (newGroupId: string) => {
    if (newGroupId === queryGroupId) return;
    try {
      const groupQueries = await queryClient.fetchQuery({
        queryKey: ['list_queries', engineId, newGroupId],
        queryFn: () => fetchListQueries(engineId!, newGroupId),
      });
      const firstQuery = groupQueries[0];
      if (firstQuery) {
        navigate({
          to: '/profile/engine/$engineId/query/$queryId',
          params: { engineId: engineId!, queryId: firstQuery.id },
        });
      }
    } catch {
      // ignore
    }
  };

  const handleQueryChange = (newQueryId: string) => {
    if (newQueryId === queryId) return;
    navigate({
      to: '/profile/engine/$engineId/query/$queryId',
      params: { engineId, queryId: newQueryId },
    });
  };

  return (
    <nav className="flex items-center gap-1.5 text-sm text-muted-foreground">
      <BreadcrumbDropdown
        label={engine}
        activeId={engineId}
        items={engines?.map(e => ({ id: e.id, label: e.instance_name ?? e.id }))}
        onSelect={handleEngineChange}
      />
      <ChevronRight className="h-3.5 w-3.5 shrink-0" />
      <BreadcrumbDropdown
        label={queryGroupName ?? queryGroupId ?? ''}
        activeId={queryGroupId ?? ''}
        items={queryGroups?.map(g => ({ id: g.id, label: g.instance_name ?? g.id }))}
        onSelect={handleQueryGroupChange}
      />
      <ChevronRight className="h-3.5 w-3.5 shrink-0" />
      <BreadcrumbDropdown
        label={queryName ?? queryId ?? ''}
        activeId={queryId ?? ''}
        items={queries?.map(q => ({ id: q.id, label: q.instance_name ?? q.id }))}
        onSelect={handleQueryChange}
      />
    </nav>
  );
}
