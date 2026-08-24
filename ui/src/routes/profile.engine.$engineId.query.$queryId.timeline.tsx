// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute } from '@tanstack/react-router';
import { QueryResourceTree } from '@/components/QueryResourceTree';
import { Route as QueryRoute } from './profile.engine.$engineId.query.$queryId';
import { useDeepLink } from '@/features/deep-link';

export const Route = createFileRoute('/profile/engine/$engineId/query/$queryId/timeline')({
  component: TimelineTab,
});

function TimelineTab() {
  const { engineId } = Route.useParams();
  const queryBundle = QueryRoute.useLoaderData();
  const deepLink = useDeepLink();
  return (
    <div className="flex min-w-0 w-full h-full min-h-[200px]">
      <QueryResourceTree
        engineId={engineId}
        queryBundle={queryBundle}
        initialZoomRange={deepLink?.initialZoomRange ?? undefined}
        seedRootExpanded={deepLink?.initialExpandedResourceIds === null}
      />
    </div>
  );
}
