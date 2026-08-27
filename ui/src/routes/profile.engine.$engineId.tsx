// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { Provider } from 'jotai';
import { useState } from 'react';
import { QueryPlan } from '@/components/QueryPlan';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@quent/components';
import { DeepLinkBoundary } from '@/features/deep-link';
import type { EntityRef, QueryBundle, ResourceTree } from '@quent/utils';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function entityRefId(ref: EntityRef): string {
  return Object.values(ref)[0]!;
}

function firstResourceId(tree: ResourceTree<EntityRef>): string | null {
  if ('Resource' in tree) {
    return entityRefId(tree.Resource);
  }
  for (const child of tree.ResourceGroup.children) {
    const resourceId = firstResourceId(child);
    if (resourceId) {
      return resourceId;
    }
  }
  return null;
}

function defaultRootResourceType(queryBundle: QueryBundle<EntityRef> | undefined): string | null {
  if (!queryBundle) {
    return null;
  }
  const resourceId = firstResourceId(queryBundle.resource_tree);
  return resourceId ? (queryBundle.entities.resources[resourceId]?.type_name ?? null) : null;
}

function ProfileLayout() {
  const { engineId } = Route.useParams();

  // Match the query layout route (covers all /query/$queryId/* children)
  const queryMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId',
    shouldThrow: false,
  });
  const queryId = queryMatch?.params?.queryId;
  const encodedState = queryMatch?.search?.s;
  const queryBundle = queryMatch?.loaderData;
  const timelineMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/timeline',
    shouldThrow: false,
  });
  const operatorsMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/operators',
    shouldThrow: false,
  });
  const activeTab = timelineMatch ? 'timeline' : operatorsMatch ? 'operators' : undefined;
  const hasQuery = queryId !== undefined;
  const isQueryReady = !hasQuery || queryMatch?.status === 'success';
  // Stripping a consumed `s` keeps the store; a different payload resets it.
  const [providerPayload, setProviderPayload] = useState(encodedState);
  if (encodedState !== undefined && encodedState !== providerPayload) {
    setProviderPayload(encodedState);
  }

  if (encodedState && !isQueryReady) {
    return <Outlet />;
  }

  return (
    <Provider key={`${engineId}:${queryId ?? ''}:${providerPayload ?? ''}`}>
      <DeepLinkBoundary
        engineId={engineId}
        queryId={queryId}
        activeTab={activeTab}
        durationSeconds={queryBundle?.duration_s ?? 0}
        defaultRootResourceType={defaultRootResourceType(queryBundle)}
        encodedState={encodedState}
        isQueryReady={isQueryReady}
      >
        <ResizablePanelGroup orientation="horizontal" className="h-full min-w-0">
          <ResizablePanel defaultSize="33%" minSize="15%" collapsible collapsedSize="0%">
            {queryId && queryId !== '' ? (
              <QueryPlan queryId={queryId} engineId={engineId} />
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                Select a query to view the execution plan
              </div>
            )}
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel
            defaultSize="67%"
            minSize="20%"
            collapsible
            collapsedSize="0%"
            className="min-w-0 overflow-x-hidden overflow-y-auto h-[calc(100vh-4rem)]"
          >
            <Outlet />
          </ResizablePanel>
        </ResizablePanelGroup>
      </DeepLinkBoundary>
    </Provider>
  );
}
