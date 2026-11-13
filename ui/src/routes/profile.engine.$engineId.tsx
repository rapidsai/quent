// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Outlet, useMatch, useNavigate } from '@tanstack/react-router';
import { Provider } from 'jotai';
import { useCallback, useMemo, useState } from 'react';
import { QueryPlan } from '@/components/QueryPlan';
import { QueryColorRegistry } from '@/components/QueryColorRegistry';
import {
  QueryToolbar,
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  transformResourceTree,
} from '@quent/components';
import { CopyLinkButton, DeepLinkBoundary } from '@/features/deep-link';
import { SavedViewsMenu } from '@/features/saved-views';
import type { SavedViewV1 } from '@/features/saved-views/savedViews.schema';
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

function ProfileLayout() {
  const { engineId } = Route.useParams();
  const navigate = useNavigate();
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
  const savedViewsScopeKey = `${engineId}:${queryId ?? ''}`;
  const [savedViewsMenuState, setSavedViewsMenuState] = useState({
    scopeKey: savedViewsScopeKey,
    open: false,
  });
  if (savedViewsMenuState.scopeKey !== savedViewsScopeKey) {
    setSavedViewsMenuState({ scopeKey: savedViewsScopeKey, open: false });
  }
  const savedViewsOpen = savedViewsMenuState.open;
  const setSavedViewsOpen = useCallback(
    (open: boolean) => setSavedViewsMenuState({ scopeKey: savedViewsScopeKey, open }),
    [savedViewsScopeKey]
  );
  // Tracks the last explicitly loaded saved view; subsequent viewport edits do not deselect it.
  const [savedViewRestore, setSavedViewRestore] = useState<{
    token: string;
    queryId: string;
    view: SavedViewV1;
  } | null>(null);
  // The view the user started from. It stays selected across tab switches and edits.
  const [selectedView, setSelectedView] = useState<{ queryId: string; viewId: string } | null>(
    null
  );
  const selectedViewId =
    selectedView && selectedView.queryId === queryId ? selectedView.viewId : undefined;
  const selectView = useCallback(
    (viewId: string) => {
      if (queryId) {
        setSelectedView({ queryId, viewId });
      }
    },
    [queryId]
  );
  const activeSavedViewRestore =
    savedViewRestore &&
    savedViewRestore.queryId === queryId &&
    savedViewRestore.view.tab === activeTab
      ? savedViewRestore
      : undefined;
  const restorePayload =
    activeSavedViewRestore?.view.payload ??
    (savedViewRestore?.queryId === queryId ? undefined : encodedState);
  const hasQuery = queryId !== undefined;
  const isQueryReady = !hasQuery || queryMatch?.status === 'success';
  // Stripping a consumed `s` keeps the store; a different payload resets it.
  const incomingProviderPayload = activeSavedViewRestore
    ? `saved:${activeSavedViewRestore.token}`
    : restorePayload;
  const [providerPayload, setProviderPayload] = useState(incomingProviderPayload);
  if (incomingProviderPayload !== undefined && incomingProviderPayload !== providerPayload) {
    setProviderPayload(incomingProviderPayload);
  }

  const loadSavedView = useCallback(
    (view: SavedViewV1) => {
      if (!queryId) {
        return;
      }
      setSavedViewRestore({ token: crypto.randomUUID(), queryId, view });
      setSelectedView({ queryId, viewId: view.id });
      const to =
        view.tab === 'timeline'
          ? '/profile/engine/$engineId/query/$queryId/timeline'
          : view.tab === 'operators'
            ? '/profile/engine/$engineId/query/$queryId/operators'
            : '/profile/engine/$engineId/query/$queryId/entities';
      void navigate({
        to,
        params: { engineId, queryId },
        search: {},
      });
    },
    [engineId, navigate, queryId]
  );

  if (!hasQuery) {
    return <Outlet />;
  }

  if (!isQueryReady) {
    return <Outlet />;
  }

  const content = (
    <DeepLinkBoundary
      engineId={engineId}
      queryId={queryId}
      activeTab={activeTab}
      initialPlanId={queryBundle?.plan_tree.id ?? ''}
      durationSeconds={queryBundle?.duration_s ?? 0}
      defaultRootResourceType={defaultRootResourceType(queryBundle)}
      operators={operators}
      encodedState={restorePayload}
      isQueryReady={isQueryReady}
      savedViewExtras={activeSavedViewRestore?.view.extras}
      restoreKind={activeSavedViewRestore ? 'saved' : 'shared'}
      onLoadSavedView={loadSavedView}
    >
      <div className="flex h-full min-w-0 flex-col">
        {queryId && (
          <QueryToolbar>
            {activeTab && (
              <SavedViewsMenu
                engineId={engineId}
                queryId={queryId}
                activeTab={activeTab}
                selectedViewId={selectedViewId}
                onSelectView={selectView}
                open={savedViewsOpen}
                onOpenChange={setSavedViewsOpen}
              />
            )}
            <CopyLinkButton />
          </QueryToolbar>
        )}
        <ResizablePanelGroup orientation="horizontal" className="min-h-0 min-w-0 flex-1">
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
            className="min-w-0 overflow-x-hidden overflow-y-auto"
          >
            <Outlet />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </DeepLinkBoundary>
  );

  return (
    <Provider key={`${engineId}:${queryId ?? ''}:${providerPayload ?? ''}`}>
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
