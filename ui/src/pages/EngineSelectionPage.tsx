// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useNavigate } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { fetchListEngines, fetchListCoordinators, fetchListQueries } from '@/services/api';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { cn } from '@/lib/utils';

export function EngineSelectionPage() {
  const navigate = useNavigate();
  const [engineId, setEngineId] = useState<string>('');
  const [coordinatorId, setCoordinatorId] = useState<string>('');

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
  };

  const handleCoordinatorChange = (value: string) => {
    setCoordinatorId(value);
  };

  const handleQuerySelect = (queryId: string) => {
    if (engineId && queryId) {
      navigate({
        to: '/profile/engine/$engineId/query/$queryId',
        params: { engineId, queryId },
      });
    }
  };

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
            <SelectTrigger>
              <SelectValue placeholder="Select Engine" />
            </SelectTrigger>
            <SelectContent>
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
                  <SelectItem key={engine.id} value={engine.id}>
                    {engine.instance_name ?? engine.id}
                  </SelectItem>
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
            <SelectTrigger>
              <SelectValue placeholder="Select Query Group" />
            </SelectTrigger>
            <SelectContent>
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
                  <SelectItem key={coordinator.id} value={coordinator.id}>
                    {coordinator.instance_name ?? coordinator.id}
                  </SelectItem>
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
          <Select onValueChange={handleQuerySelect}>
            <SelectTrigger>
              <SelectValue placeholder="Select Query" />
            </SelectTrigger>
            <SelectContent>
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
                  <SelectItem key={query.id} value={query.id}>
                    {query.instance_name ?? query.id}
                  </SelectItem>
                ))
              )}
            </SelectContent>
          </Select>
        </div>
      </div>
    </div>
  );
}
