// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { QueryResourceTree } from '@/components/timeline-tree';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';
import { useViewState } from '@/features/view-state';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/timeline')({
  component: TimelineTab,
});

function TimelineTab() {
  const { engineId } = Route.useParams();
  const queryBundle = QueryRoute.useLoaderData();
  const viewState = useViewState();
  return (
    <div className="flex min-w-0 w-full h-full min-h-[200px] pt-2">
      <QueryResourceTree
        engineId={engineId}
        queryBundle={queryBundle}
        initialZoomRange={viewState?.initialZoomRange ?? undefined}
        seedRootExpanded={viewState?.initialExpandedResourceIds === null}
      />
    </div>
  );
}
