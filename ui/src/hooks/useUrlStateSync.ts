// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { useAtomValue } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import { useNavigate } from '@tanstack/react-router';
import { selectedPlanIdAtom, selectedNodeIdsAtom } from '@/atoms/dag';
import { debouncedZoomRangeAtom, hideTasksAtom } from '@/atoms/timeline';

export interface QueryIndexSearch {
  planId?: string;
  operatorId?: string;
  zoomStart?: number;
  zoomEnd?: number;
  hideTasks?: boolean;
}

/**
 * Syncs a fixed set of scalar UI atoms with the URL search params for the query index route.
 *
 * On mount: seeds atoms from URL values (via useHydrateAtoms) so deep links restore state.
 * On change: writes updated atom values back to the URL using replace navigation so the
 * browser history stack is not polluted on every zoom gesture or plan selection.
 *
 * Zoom range is NOT seeded here — it is passed as initialZoom to QueryResourceTree which
 * owns the useHydrateAtoms call for timeline atoms. This avoids a timing conflict where
 * QueryResourceTree would override whatever zoom this hook tried to hydrate.
 */
export function useUrlStateSync(search: QueryIndexSearch) {
  useHydrateAtoms([
    [selectedPlanIdAtom, search.planId ?? ''],
    [selectedNodeIdsAtom, search.operatorId ? new Set([search.operatorId]) : new Set<string>()],
    [hideTasksAtom, search.hideTasks ?? false],
  ]);

  const planId = useAtomValue(selectedPlanIdAtom);
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const zoomRange = useAtomValue(debouncedZoomRangeAtom);
  const hideTasks = useAtomValue(hideTasksAtom);

  const operatorId = selectedNodeIds.size > 0 ? [...selectedNodeIds][0] : undefined;

  // Scoping navigate to this route gives TanStack Router the search type context it needs
  // to type-check the search updater function correctly.
  const navigate = useNavigate({ from: '/profile/engine/$engineId/query/$queryId/' });

  useEffect(() => {
    // zoomRange stays at { start: 0, end: 0 } until QueryResourceTree's useHydrateAtoms
    // runs during its render. Skip URL writes until zoom is properly initialized.
    if (zoomRange.end === 0) return;

    void navigate({
      search: prev => ({
        ...prev,
        planId: planId || undefined,
        operatorId,
        zoomStart: zoomRange.start,
        zoomEnd: zoomRange.end,
        hideTasks: hideTasks || undefined,
      }),
      replace: true,
    });
  }, [planId, operatorId, zoomRange.start, zoomRange.end, hideTasks, navigate]);
}
