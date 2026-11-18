// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { Provider } from 'jotai';
import { useMemo, useState } from 'react';
import { QueryPlan } from '@/components/QueryPlan';
import { QueryColorRegistry } from '@/components/QueryColorRegistry';
import {
  QueryToolbar,
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  transformResourceTree,
} from '@quent/components';
import {
  CopyLinkButton,
  ViewStateBoundary,
  ViewStatePersistence,
  UrlViewState,
  applicationViewStateRepository,
  createUrlViewStateSource,
  type ViewStateScope,
  useResolvedViewState,
} from '@/features/view-state';
import { usePanelLayout } from '@/hooks/usePanelLayout';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';
import { type EntityRef, type PaletteTheme, type QueryBundle } from '@quent/utils';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function defaultRootResourceType(queryBundle: QueryBundle<EntityRef> | undefined): string | null {
  if (!queryBundle) {
    return null;
  }
  const root = transformResourceTree(queryBundle.entities, queryBundle.resource_tree);
  return (
    root.availableResourceTypes?.[0] ??
    (root.entity && 'type_name' in root.entity ? root.entity.type_name : null)
  );
}

const PROFILE_PANEL_IDS = ['query-plan', 'query-tabs'];

function ProfileLayout() {
  const { engineId } = Route.useParams();
  const { theme } = useTheme();
  const paletteTheme: PaletteTheme = theme === THEME_DARK ? 'dark' : 'light';

  // Match the query layout route (covers all /query/$queryId/* children)
  const queryMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId',
    shouldThrow: false,
  });
  const queryId = queryMatch?.params?.queryId;
  const encodedState = queryMatch?.search?.s;
  const queryBundle = queryMatch?.loaderData;
  const operators = useMemo(
    () => (queryBundle ? Object.values(queryBundle.entities.operators) : []),
    [queryBundle]
  );
  const timelineMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/timeline',
    shouldThrow: false,
  });
  const operatorsMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/operators',
    shouldThrow: false,
  });
  const entitiesMatch = useMatch({
    from: '/profile/engine/$engineId/query/$queryId/entities',
    shouldThrow: false,
  });
  const activeTab = timelineMatch
    ? 'timeline'
    : operatorsMatch
      ? 'operators'
      : entitiesMatch
        ? 'entities'
        : undefined;
  const hasQuery = queryId !== undefined;
  const isQueryReady = !hasQuery || queryMatch?.status === 'success';
  const providerScopeKey = queryId ? `${engineId}:${queryId}` : null;
  const [providerState, setProviderState] = useState({
    scopeKey: providerScopeKey,
    urlPayload: encodedState,
  });
  if (providerState.scopeKey !== providerScopeKey) {
    setProviderState({ scopeKey: providerScopeKey, urlPayload: encodedState });
  } else if (encodedState !== undefined && encodedState !== providerState.urlPayload) {
    setProviderState({ scopeKey: providerScopeKey, urlPayload: encodedState });
  }
  const urlPayload =
    providerState.scopeKey === providerScopeKey ? providerState.urlPayload : encodedState;
  const stateScope = useMemo<ViewStateScope | null>(
    () => (queryId && activeTab ? { engineId, queryId, tab: activeTab } : null),
    [activeTab, engineId, queryId]
  );
  const stateSources = useMemo(
    () => [createUrlViewStateSource(urlPayload), applicationViewStateRepository],
    [urlPayload]
  );
  const stateResolution = useResolvedViewState({
    enabled: isQueryReady && stateScope !== null,
    resolutionKey: queryId ? `${engineId}:${queryId}:${urlPayload ?? ''}` : null,
    scope: stateScope,
    sources: stateSources,
  });
  const resolvedState =
    stateResolution.status === 'ready' ? (stateResolution.encodedState ?? undefined) : undefined;

  const layout = usePanelLayout('profile-split', PROFILE_PANEL_IDS);

  if (!hasQuery) {
    return <Outlet />;
  }

  if (!isQueryReady) {
    return <Outlet />;
  }
  if (stateResolution.status === 'loading') {
    return <Outlet />;
  }

  const content = (
    <ViewStateBoundary
      engineId={engineId}
      queryId={queryId}
      activeTab={activeTab}
      initialPlanId={queryBundle?.plan_tree.id ?? ''}
      durationSeconds={queryBundle?.duration_s ?? 0}
      defaultRootResourceType={defaultRootResourceType(queryBundle)}
      operators={operators}
      encodedState={resolvedState}
      isQueryReady={isQueryReady}
    >
      <div className="flex h-full min-w-0 flex-col">
        <UrlViewState active={stateResolution.sourceId === 'url'} />
        {activeTab && (
          <ViewStatePersistence
            repository={applicationViewStateRepository}
            engineId={engineId}
            queryId={queryId}
            tab={activeTab}
            restoring={stateResolution.sourceId === applicationViewStateRepository.id}
          />
        )}
        {queryId && (
          <QueryToolbar>
            <CopyLinkButton />
          </QueryToolbar>
        )}
        <ResizablePanelGroup
          orientation="horizontal"
          className="min-h-0 min-w-0 flex-1"
          {...layout}
        >
          <ResizablePanel
            id="query-plan"
            defaultSize="33%"
            minSize="15%"
            collapsible
            collapsedSize="0%"
          >
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
            id="query-tabs"
            defaultSize="67%"
            minSize="20%"
            collapsible
            collapsedSize="0%"
            className="min-w-0 overflow-x-hidden overflow-y-auto"
          >
            <Outlet />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </ViewStateBoundary>
  );

  return (
    <Provider key={`${engineId}:${queryId ?? ''}:${urlPayload ?? ''}`}>
      {queryBundle ? (
        <QueryColorRegistry queryBundle={queryBundle} paletteTheme={paletteTheme}>
          {content}
        </QueryColorRegistry>
      ) : (
        content
      )}
    </Provider>
  );
}
