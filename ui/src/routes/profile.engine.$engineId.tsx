// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createFileRoute, Outlet, useMatch } from '@tanstack/react-router';
import { Provider } from 'jotai';
import { useMemo, useState, type ReactNode } from 'react';
import { QueryPlan } from '@/components/QueryPlan';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@quent/components';
import { COLOR_REGISTRY_KEYS, useHydrateColorRegistry } from '@quent/hooks';
import { DeepLinkBoundary } from '@/features/deep-link';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';
import {
  createColorRegistry,
  createColorRegistryEntry,
  getColorRegistryPalettes,
  unpackEntityRef,
  type ColorRegistry,
  type EntityRef,
  type PaletteTheme,
  type QueryBundle,
  type ResourceTree,
} from '@quent/utils';

export const Route = createFileRoute('/profile/engine/$engineId')({
  component: ProfileLayout,
});

function entityRefId(ref: EntityRef): string {
  return unpackEntityRef(ref).id;
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

function QueryColorRegistry({
  queryBundle,
  paletteTheme,
  children,
}: {
  queryBundle: QueryBundle<EntityRef>;
  paletteTheme: PaletteTheme;
  children: ReactNode;
}) {
  const registry = useMemo<ColorRegistry>(() => {
    const resourceTypes = Object.values(queryBundle.entities.resource_types);
    const fsmTypes = Object.values(queryBundle.entities.fsm_types);
    const fsmStates = fsmTypes.flatMap(type => type.states.map(state => state.name));
    const palettes = getColorRegistryPalettes(paletteTheme);

    return createColorRegistry([
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.OPERATOR_TYPES,
        queryBundle.unique_operator_names,
        palettes[COLOR_REGISTRY_KEYS.OPERATOR_TYPES]
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.RESOURCE_TYPES,
        resourceTypes,
        palettes[COLOR_REGISTRY_KEYS.RESOURCE_TYPES],
        type => type.name
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.FSM_TYPES,
        fsmTypes,
        palettes[COLOR_REGISTRY_KEYS.FSM_TYPES],
        type => type.name
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.CAPACITIES,
        resourceTypes.flatMap(type => type.capacities.map(capacity => capacity.name)),
        palettes[COLOR_REGISTRY_KEYS.CAPACITIES]
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.FSM_STATES,
        fsmStates,
        palettes[COLOR_REGISTRY_KEYS.FSM_STATES]
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.DATA_FLOW_STATES,
        [],
        palettes[COLOR_REGISTRY_KEYS.DATA_FLOW_STATES]
      ),
      createColorRegistryEntry(
        COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS,
        [],
        palettes[COLOR_REGISTRY_KEYS.DATA_FLOW_DIMENSIONS]
      ),
    ]);
  }, [paletteTheme, queryBundle]);
  useHydrateColorRegistry(registry);
  return children;
}

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
  // Stripping a consumed `s` keeps the store; a different payload resets it.
  const [providerPayload, setProviderPayload] = useState(encodedState);
  if (encodedState !== undefined && encodedState !== providerPayload) {
    setProviderPayload(encodedState);
  }

  if (hasQuery && !isQueryReady) {
    return <Outlet />;
  }

  const content = (
    <DeepLinkBoundary
      engineId={engineId}
      queryId={queryId}
      activeTab={activeTab}
      durationSeconds={queryBundle?.duration_s ?? 0}
      defaultRootResourceType={defaultRootResourceType(queryBundle)}
      operators={operators}
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
