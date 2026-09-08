// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useNavigate } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { fetchListEngines, fetchListCoordinators, fetchListQueries } from '@quent/client';
import {
  DataText,
  HoverCard,
  HoverCardTrigger,
  OverflowHoverCardContent,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  useOverflowHoverCard,
} from '@quent/components';
import { cn } from '@quent/utils';

function OverflowingSelectTrigger({
  label,
  placeholder,
}: {
  label: string | undefined;
  placeholder: string;
}) {
  const valueRef = useRef<HTMLSpanElement>(null);
  const { open, handlePointerEnter, handlePointerLeave } = useOverflowHoverCard(valueRef, !!label);

  return (
    <HoverCard open={open}>
      <HoverCardTrigger asChild>
        <SelectTrigger
          onPointerEnter={handlePointerEnter}
          onPointerLeave={handlePointerLeave}
          onBlur={handlePointerLeave}
        >
          <span ref={valueRef} className="min-w-0 flex-1 truncate text-left">
            <SelectValue placeholder={placeholder}>{label}</SelectValue>
          </span>
        </SelectTrigger>
      </HoverCardTrigger>
      {label && <OverflowHoverCardContent label={label} />}
    </HoverCard>
  );
}

function OverflowingSelectItem({ value, label }: { value: string; label: string }) {
  const labelRef = useRef<HTMLSpanElement>(null);
  const { open, handlePointerEnter, handlePointerLeave } = useOverflowHoverCard(labelRef);

  return (
    <HoverCard open={open}>
      <HoverCardTrigger asChild>
        <SelectItem
          value={value}
          className="min-w-0"
          onPointerEnter={handlePointerEnter}
          onPointerLeave={handlePointerLeave}
        >
          <span ref={labelRef} className="block w-full min-w-0 truncate">
            <DataText>{label}</DataText>
          </span>
        </SelectItem>
      </HoverCardTrigger>
      <OverflowHoverCardContent label={label} />
    </HoverCard>
  );
}

export function EngineSelectionPage() {
  const navigate = useNavigate();
  const [engineId, setEngineId] = useState<string>('');
  const [coordinatorId, setCoordinatorId] = useState<string>('');
  const [queryId, setQueryId] = useState<string>('');

  const enginesList = useQuery({
    queryKey: ['list_engines'],
    queryFn: fetchListEngines,
  });

  const coordinatorsList = useQuery({
    queryKey: ['list_coordinators', engineId],
    queryFn: () => (engineId ? fetchListCoordinators(engineId) : Promise.resolve([])),
    enabled: !!engineId,
  });

  const queryList = useQuery({
    queryKey: ['list_queries', engineId, coordinatorId],
    queryFn: () =>
      engineId && coordinatorId ? fetchListQueries(engineId, coordinatorId) : Promise.resolve([]),
    enabled: !!engineId && !!coordinatorId,
  });

  const handleEngineChange = (value: string) => {
    setEngineId(value);
    setCoordinatorId('');
    setQueryId('');
  };

  const handleCoordinatorChange = (value: string) => {
    setCoordinatorId(value);
    setQueryId('');
  };

  const handleQuerySelect = (queryId: string) => {
    setQueryId(queryId);
    if (engineId && queryId) {
      navigate({
        to: '/profile/engine/$engineId/query/$queryId',
        params: { engineId, queryId },
        search: {},
      });
    }
  };

  const engineOptions = [
    ...(enginesList.data?.map(engine => ({
      id: engine.id,
      name: engine.instance_name ?? engine.id,
    })) ?? []),
  ];
  const coordinatorOptions = [
    ...(coordinatorsList.data?.map(coordinator => ({
      id: coordinator.id,
      name: coordinator.instance_name ?? coordinator.id,
    })) ?? []),
  ];
  const queryOptions = [
    ...(queryList.data?.map(query => ({
      id: query.id,
      name: query.instance_name ?? query.id,
    })) ?? []),
  ];
  const engineLabel = engineOptions.find(engine => engine.id === engineId)?.name;
  const coordinatorLabel = coordinatorOptions.find(
    coordinator => coordinator.id === coordinatorId
  )?.name;
  const queryLabel = queryOptions.find(query => query.id === queryId)?.name;

  return (
    <div className="flex flex-col items-center justify-center h-full min-h-[400px] space-y-6">
      <h1 className="text-2xl font-semibold">Query Profiler</h1>
      <p className="text-muted-foreground text-center max-w-md">
        Select an engine, coordinator, and query to view execution plans and profiles.
      </p>
      <div className="w-full max-w-xs space-y-4">
        {/* Engine Selection */}
        <div>
          <label htmlFor="engineId" className="block text-sm font-medium mb-1">
            Engine
          </label>
          <Select value={engineId} onValueChange={handleEngineChange}>
            <OverflowingSelectTrigger label={engineLabel ?? engineId} placeholder="Select Engine" />
            <SelectContent className="max-h-64 w-[var(--radix-select-trigger-width)] max-w-[var(--radix-select-trigger-width)] overflow-y-auto">
              {enginesList.isLoading ? (
                <SelectItem value="_loading" disabled>
                  Loading engines...
                </SelectItem>
              ) : enginesList.data?.length === 0 ? (
                <SelectItem value="_empty" disabled>
                  No engines available
                </SelectItem>
              ) : (
                enginesList.data?.map(engine => (
                  <OverflowingSelectItem
                    key={engine.id}
                    value={engine.id}
                    label={engine.instance_name ?? engine.id}
                  />
                ))
              )}
            </SelectContent>
          </Select>
        </div>

        {/* Coordinator Selection */}
        <div className={cn(engineId && 'visible', !engineId && 'invisible')}>
          <label htmlFor="coordinatorId" className="block text-sm font-medium mb-1">
            Query Group
          </label>
          <Select value={coordinatorId} onValueChange={handleCoordinatorChange}>
            <OverflowingSelectTrigger
              label={coordinatorLabel ?? coordinatorId}
              placeholder="Select Query Group"
            />
            <SelectContent className="max-h-64 w-[var(--radix-select-trigger-width)] max-w-[var(--radix-select-trigger-width)] overflow-y-auto">
              {coordinatorsList.isLoading ? (
                <SelectItem value="_loading" disabled>
                  Loading Query Groups...
                </SelectItem>
              ) : coordinatorsList.data?.length === 0 ? (
                <SelectItem value="_empty" disabled>
                  No Query Groups available
                </SelectItem>
              ) : (
                coordinatorsList.data?.map(coordinator => (
                  <OverflowingSelectItem
                    key={coordinator.id}
                    value={coordinator.id}
                    label={coordinator.instance_name ?? coordinator.id}
                  />
                ))
              )}
            </SelectContent>
          </Select>
        </div>

        {/* Query Selection */}
        <div className={cn(coordinatorId && 'visible', !coordinatorId && 'invisible')}>
          <label htmlFor="queryId" className="block text-sm font-medium mb-1">
            Query
          </label>
          <Select value={queryId} onValueChange={handleQuerySelect}>
            <OverflowingSelectTrigger label={queryLabel} placeholder="Select Query" />
            <SelectContent className="max-h-64 w-[var(--radix-select-trigger-width)] max-w-[var(--radix-select-trigger-width)] overflow-y-auto">
              {queryList.isLoading ? (
                <SelectItem value="_loading" disabled>
                  Loading queries...
                </SelectItem>
              ) : queryList.data?.length === 0 ? (
                <SelectItem value="_empty" disabled>
                  No queries available
                </SelectItem>
              ) : (
                queryList.data?.map(query => (
                  <OverflowingSelectItem
                    key={query.id}
                    value={query.id}
                    label={query.instance_name ?? query.id}
                  />
                ))
              )}
            </SelectContent>
          </Select>
        </div>
      </div>
    </div>
  );
}
