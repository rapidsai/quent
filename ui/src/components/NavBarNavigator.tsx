// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMatch, useNavigate } from '@tanstack/react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRef } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import {
  DataText,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  HoverCard,
  HoverCardTrigger,
  OverflowHoverCardContent,
  OverflowingItemLabel,
  useOverflowHoverCard,
} from '@quent/components';
import { cn } from '@quent/utils';
import {
  queryBundleQueryOptions,
  fetchListEngines,
  fetchListCoordinators,
  fetchListQueries,
} from '@quent/client';

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
  const labelRef = useRef<HTMLSpanElement>(null);
  const { open, handlePointerEnter, handlePointerLeave } = useOverflowHoverCard(labelRef);

  return (
    <HoverCard open={open}>
      <DropdownMenu>
        <HoverCardTrigger asChild>
          <DropdownMenuTrigger asChild>
            <button
              className="-mx-1.5 flex min-w-0 max-w-40 cursor-pointer items-center gap-0.5 rounded-sm px-1.5 py-0.5 transition-colors hover:bg-accent hover:text-foreground md:max-w-48 xl:max-w-64"
              onPointerEnter={handlePointerEnter}
              onPointerLeave={handlePointerLeave}
              onBlur={handlePointerLeave}
            >
              <span ref={labelRef} className="min-w-0 truncate">
                <DataText>{label}</DataText>
              </span>
              <ChevronDown className="h-3 w-3 shrink-0 opacity-50" />
            </button>
          </DropdownMenuTrigger>
        </HoverCardTrigger>
        <DropdownMenuContent align="start" className="max-h-64 w-max max-w-64 overflow-y-auto">
          {items?.map(item => (
            <DropdownMenuItem
              key={item.id}
              onSelect={() => onSelect(item.id)}
              className={cn('min-w-0', item.id === activeId && 'bg-accent font-semibold')}
            >
              <OverflowingItemLabel label={item.label} />
            </DropdownMenuItem>
          ))}
          {(!items || items.length === 0) && <DropdownMenuItem disabled>No items</DropdownMenuItem>}
        </DropdownMenuContent>
      </DropdownMenu>
      <OverflowHoverCardContent label={label} side="bottom" />
    </HoverCard>
  );
}

export function NavBarNavigator() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  // Match the layout route — satisfied by any child route (timeline, operators,
  // node/$nodeId, index, …) without needing a per-leaf match here.
  const queryLayoutMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId',
    shouldThrow: false,
  });

  const engineId = queryLayoutMatch?.params?.engineId;
  const queryId = queryLayoutMatch?.params?.queryId;

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

  if (!queryBundle || !engineId) {
    return null;
  }

  const engineItems =
    engines?.map(engine => ({
      id: engine.id,
      label: engine.instance_name ?? engine.id,
    })) ?? [];
  const queryGroupItems =
    queryGroups?.map(queryGroup => ({
      id: queryGroup.id,
      label: queryGroup.instance_name ?? queryGroup.id,
    })) ?? [];
  const queryItems =
    queries?.map(query => ({
      id: query.id,
      label: query.instance_name ?? query.id,
    })) ?? [];
  const engine = queryBundle.entities.engine.instance_name ?? queryBundle.entities.engine.id;
  const queryGroupName =
    queryBundle.entities.query_group.instance_name ?? queryBundle.entities.query_group.id;
  const queryName = queryBundle.entities.query.instance_name ?? queryBundle.entities.query.id;

  const handleEngineChange = async (newEngineId: string) => {
    if (newEngineId === engineId) {
      return;
    }
    try {
      const groups = await queryClient.fetchQuery({
        queryKey: ['list_coordinators', newEngineId],
        queryFn: () => fetchListCoordinators(newEngineId),
      });
      const firstGroup = groups[0];
      if (!firstGroup) {
        return;
      }
      const groupQueries = await queryClient.fetchQuery({
        queryKey: ['list_queries', newEngineId, firstGroup.id],
        queryFn: () => fetchListQueries(newEngineId, firstGroup.id),
      });
      const firstQuery = groupQueries[0];
      if (firstQuery) {
        navigate({
          to: '/profile/engine/$engineId/query/$queryId',
          params: { engineId: newEngineId, queryId: firstQuery.id },
          search: {},
        });
      }
    } catch {
      // ignore
    }
  };

  const handleQueryGroupChange = async (newGroupId: string) => {
    if (newGroupId === queryGroupId) {
      return;
    }
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
          search: {},
        });
      }
    } catch {
      // ignore
    }
  };

  const handleQueryChange = (newQueryId: string) => {
    if (newQueryId === queryId) {
      return;
    }
    navigate({
      to: '/profile/engine/$engineId/query/$queryId',
      params: { engineId, queryId: newQueryId },
      search: {},
    });
  };

  return (
    <nav className="flex min-w-0 max-w-full items-center gap-1.5 text-sm text-muted-foreground">
      <BreadcrumbDropdown
        label={engine}
        activeId={engineId}
        items={engineItems}
        onSelect={handleEngineChange}
      />
      <ChevronRight className="h-3.5 w-3.5 shrink-0" />
      <BreadcrumbDropdown
        label={queryGroupName ?? queryGroupId ?? ''}
        activeId={queryGroupId ?? ''}
        items={queryGroupItems}
        onSelect={handleQueryGroupChange}
      />
      <ChevronRight className="h-3.5 w-3.5 shrink-0" />
      <BreadcrumbDropdown
        label={queryName}
        activeId={queryId ?? ''}
        items={queryItems}
        onSelect={handleQueryChange}
      />
    </nav>
  );
}
