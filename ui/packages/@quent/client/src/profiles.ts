// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import type { Engine, Query, QueryGroup } from '@quent/utils';
import { fetchListEngines, fetchListCoordinators, fetchListQueries } from './api';
import { DEFAULT_STALE_TIME } from './constants';

/** A single query profile flattened with its engine and query-group context. */
export interface ProfileRow {
  query: Query;
  engine: Engine;
  queryGroup: QueryGroup | null;
}

export type ProfileStatus = 'completed' | 'executing' | 'planning' | 'unknown';

/** Derive a coarse lifecycle status from the query's phase timestamps. */
export function getProfileStatus(query: Query): ProfileStatus {
  if (query.completed_s != null) return 'completed';
  if (query.executing_s != null) return 'executing';
  if (query.planning_s != null) return 'planning';
  return 'unknown';
}

/**
 * Aggregate every query across all engines and query groups into a flat list.
 *
 * The backend has no "list all profiles" endpoint yet (see the `pagination`
 * TODOs in `domains/query_engine/server/src/ui.rs`), so this fans out over the
 * hierarchical list endpoints client-side. Replace the body with a single
 * search/list call once the BE exposes one — callers only depend on
 * {@link ProfileRow}.
 */
export async function fetchAllProfiles(): Promise<ProfileRow[]> {
  const engines = await fetchListEngines();

  const perEngine = await Promise.all(
    engines.map(async engine => {
      const groups = await fetchListCoordinators(engine.id).catch(() => [] as QueryGroup[]);
      const perGroup = await Promise.all(
        groups.map(async group => {
          const queries = await fetchListQueries(engine.id, group.id).catch(() => [] as Query[]);
          return queries.map<ProfileRow>(query => ({ query, engine, queryGroup: group }));
        })
      );
      return perGroup.flat();
    })
  );

  return perEngine.flat();
}

export const allProfilesQueryOptions = (options?: { staleTime?: number }) =>
  queryOptions({
    queryKey: ['all_profiles'],
    queryFn: fetchAllProfiles,
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });

export const useAllProfiles = (options?: { staleTime?: number }) =>
  useQuery(allProfilesQueryOptions(options));
