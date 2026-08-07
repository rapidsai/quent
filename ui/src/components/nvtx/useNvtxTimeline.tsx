// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQueries, useQuery } from '@tanstack/react-query';
import { Activity } from 'lucide-react';
import {
  engineContextsQueryOptions,
  nvtxCatalogQueryOptions,
  nvtxViewportQueryOptions,
} from '@quent/client';
import { useDebouncedZoomRange } from '@quent/hooks';
import {
  NvtxTimelineControls,
  type NvtxCatalogControlEntry,
  type NvtxStatisticsControlEntry,
} from '@quent/components';
import type {
  EntityTypeValue,
  NvtxCatalog,
  NvtxLane,
  NvtxDomainSelection,
  NvtxViewportResponse,
} from '@quent/utils';
import { nvtxViewportFromZoom } from './nvtxTimeline.utils';
import {
  NVTX_DOMAIN_ROW_TYPE,
  NVTX_LANE_ROW_TYPE,
  NVTX_ROOT_ROW_TYPE,
  NVTX_STATUS_ROW_TYPE,
  type NvtxTimelineAdapter,
  type NvtxTimelinePlacement,
  type NvtxTimelineTreeItem,
} from './nvtxTimeline.types';

const EMPTY_ENTITY = {} as EntityTypeValue;

function selectAll(catalog: NvtxCatalog): NvtxDomainSelection[] {
  return catalog.domains.flatMap(domain => {
    const categoryIds = domain.categories.map(category => category.category_id);
    if (categoryIds.length === 0 && !domain.has_uncategorized) return [];
    return [
      {
        domain_id: domain.domain_id,
        category_ids: categoryIds,
        include_uncategorized: domain.has_uncategorized,
      },
    ];
  });
}

function contextLabel(contextId: string): string {
  return `context ${contextId.slice(0, 8)}`;
}

function statusItem(
  id: string,
  state: 'loading' | 'empty' | 'error',
  label: string,
  retry?: () => void
): NvtxTimelineTreeItem {
  return {
    id,
    type: NVTX_STATUS_ROW_TYPE,
    entity: EMPTY_ENTITY,
    nvtx: { kind: 'status', state, label, retry },
  };
}

function responseDomainItems(
  contextId: string,
  response: NvtxViewportResponse,
  includeContextLabel: boolean
): NvtxTimelineTreeItem[] {
  return response.domains.map(domain => {
    const groupedLanes = new Map<string, NvtxLane[]>();
    for (const lane of domain.lanes) {
      let groupKey: string = lane.identity.kind;
      if (lane.identity.kind === 'thread') {
        groupKey = `thread:${lane.identity.thread_id}`;
      }
      const group = groupedLanes.get(groupKey);
      if (group) group.push(lane);
      else groupedLanes.set(groupKey, [lane]);
    }
    const children = Array.from(groupedLanes.values()).map(lanes => {
      lanes.sort((left, right) => {
        const leftDepth = left.identity.kind === 'thread' ? left.identity.depth : 0;
        const rightDepth = right.identity.kind === 'thread' ? right.identity.depth : 0;
        return leftDepth - rightDepth;
      });
      const primaryLane = lanes[0]!;
      return {
        id: `nvtx:${contextId}:lane:${primaryLane.id}`,
        type: NVTX_LANE_ROW_TYPE,
        entity: EMPTY_ENTITY,
        nvtx: {
          kind: 'lane',
          label: primaryLane.label,
          lanes,
        },
      } satisfies NvtxTimelineTreeItem;
    });
    return {
      id: `nvtx:${contextId}:domain:${domain.domain_id.toString()}`,
      type: NVTX_DOMAIN_ROW_TYPE,
      entity: EMPTY_ENTITY,
      nvtx: {
        kind: 'domain',
        label: includeContextLabel ? `${domain.name} · ${contextLabel(contextId)}` : domain.name,
        color: domain.color,
      },
      children,
    } satisfies NvtxTimelineTreeItem;
  });
}

export function useNvtxTimeline({
  engineId,
  queryStartUnixNs,
  resourceIds,
  rootResourceId,
}: {
  engineId: string;
  queryStartUnixNs: bigint;
  resourceIds: ReadonlySet<string>;
  rootResourceId: string;
}): NvtxTimelineAdapter {
  const zoomRange = useDebouncedZoomRange();
  const contextsQuery = useQuery(engineContextsQueryOptions(engineId));
  const contextResources = useMemo(
    () => contextsQuery.data?.context_resources ?? {},
    [contextsQuery.data?.context_resources]
  );
  const contextIds = useMemo(
    () =>
      Object.keys(contextResources).filter(contextId => {
        const owners = contextResources[contextId] ?? [];
        return (
          owners.length === 0 ||
          owners.includes(engineId) ||
          owners.some(resourceId => resourceIds.has(resourceId))
        );
      }),
    [contextResources, engineId, resourceIds]
  );
  const catalogQueries = useQueries({
    queries: contextIds.map(contextId => nvtxCatalogQueryOptions(contextId, queryStartUnixNs)),
  });
  const [selections, setSelections] = useState<Record<string, NvtxDomainSelection[]>>({});
  const [debouncedSelections, setDebouncedSelections] = useState<
    Record<string, NvtxDomainSelection[]>
  >({});

  useEffect(() => {
    setSelections(previous => {
      const currentContextIds = new Set(contextIds);
      let changed = Object.keys(previous).some(contextId => !currentContextIds.has(contextId));
      const next: Record<string, NvtxDomainSelection[]> = {};
      contextIds.forEach((contextId, index) => {
        const catalog = catalogQueries[index]?.data;
        if (previous[contextId] !== undefined) {
          next[contextId] = previous[contextId];
        } else if (catalog) {
          next[contextId] = selectAll(catalog);
          changed = true;
        }
      });
      return changed ? next : previous;
    });
  }, [catalogQueries, contextIds]);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedSelections(selections), 150);
    return () => window.clearTimeout(timer);
  }, [selections]);

  const viewport = useMemo(() => nvtxViewportFromZoom(zoomRange), [zoomRange]);
  const viewportQueries = useQueries({
    queries: contextIds.map((contextId, index) => {
      const catalog = catalogQueries[index]?.data;
      const selection = debouncedSelections[contextId];
      return nvtxViewportQueryOptions(
        contextId,
        queryStartUnixNs,
        { viewport, selections: selection ?? [] },
        { enabled: catalog != null && selection !== undefined }
      );
    }),
  });

  const setContextSelection = useCallback((contextId: string, next: NvtxDomainSelection[]) => {
    setSelections(previous => ({ ...previous, [contextId]: next }));
  }, []);

  return useMemo(() => {
    if (contextsQuery.isSuccess && contextIds.length === 0) {
      return { placements: [], initiallyExpandedIds: [], controls: null };
    }
    const allCatalogsAbsent =
      contextIds.length > 0 &&
      catalogQueries.every(query => query.isSuccess && query.data === null);
    if (allCatalogsAbsent) {
      return { placements: [], initiallyExpandedIds: [], controls: null };
    }

    const placements: NvtxTimelinePlacement[] = [];
    const initiallyExpandedIds: string[] = [];
    const catalogControls: NvtxCatalogControlEntry[] = [];
    const statisticsControls: NvtxStatisticsControlEntry[] = [];

    const placeGroup = (contextId: string, children: NvtxTimelineTreeItem[]) => {
      const matchingResources = (contextResources[contextId] ?? []).filter(resourceId =>
        resourceIds.has(resourceId)
      );
      const parentId = matchingResources.length === 1 ? matchingResources[0]! : rootResourceId;
      const group: NvtxTimelineTreeItem = {
        id: `nvtx:${contextId}:root`,
        type: NVTX_ROOT_ROW_TYPE,
        entity: EMPTY_ENTITY,
        icon: Activity,
        nvtx: {
          kind: 'root',
          label: contextIds.length > 1 ? `NVTX · ${contextLabel(contextId)}` : 'NVTX',
        },
        children,
      };
      placements.push({ parentId, item: group });
      initiallyExpandedIds.push(
        group.id,
        ...children.filter(item => item.nvtx?.kind === 'domain').map(item => item.id)
      );
    };

    if (contextsQuery.isPending) {
      placeGroup('contexts', [
        statusItem('nvtx:loading-contexts', 'loading', 'Loading NVTX lanes…'),
      ]);
    } else if (contextsQuery.isError) {
      placeGroup('contexts', [
        statusItem(
          'nvtx:error-contexts',
          'error',
          "Couldn't load NVTX contexts. Try again.",
          () => {
            void contextsQuery.refetch();
          }
        ),
      ]);
    }

    contextIds.forEach((contextId, index) => {
      const catalogQuery = catalogQueries[index];
      const viewportQuery = viewportQueries[index];
      const children: NvtxTimelineTreeItem[] = [];
      if (!catalogQuery || catalogQuery.isPending) {
        children.push(
          statusItem(`nvtx:${contextId}:catalog-loading`, 'loading', 'Loading NVTX lanes…')
        );
        placeGroup(contextId, children);
        return;
      }
      if (catalogQuery.isError) {
        children.push(
          statusItem(
            `nvtx:${contextId}:catalog-error`,
            'error',
            `Couldn't load NVTX data for ${contextLabel(contextId)}. Try again.`,
            () => void catalogQuery.refetch()
          )
        );
        placeGroup(contextId, children);
        return;
      }
      if (catalogQuery.data === null) return;
      catalogControls.push({
        contextId,
        label: contextLabel(contextId),
        catalog: catalogQuery.data,
      });
      if (!viewportQuery || (viewportQuery.isPending && viewportQuery.data === undefined)) {
        children.push(
          statusItem(`nvtx:${contextId}:viewport-loading`, 'loading', 'Loading NVTX lanes…')
        );
        placeGroup(contextId, children);
        return;
      }
      if (viewportQuery.isError && viewportQuery.data === undefined) {
        children.push(
          statusItem(
            `nvtx:${contextId}:viewport-error`,
            'error',
            `Couldn't load NVTX data for ${contextLabel(contextId)}. Try again.`,
            () => void viewportQuery.refetch()
          )
        );
        placeGroup(contextId, children);
        return;
      }
      if (viewportQuery.data) {
        const domainItems = responseDomainItems(contextId, viewportQuery.data, false);
        const renderedItems = domainItems.reduce(
          (total, domain) => total + (domain.children?.length ?? 0),
          0
        );
        children.push(...domainItems);
        statisticsControls.push({
          contextId,
          label: contextLabel(contextId),
          statistics: viewportQuery.data.statistics,
        });
        if (viewportQuery.isError) {
          children.push(
            statusItem(
              `nvtx:${contextId}:viewport-refetch-error`,
              'error',
              `Couldn't refresh NVTX data for ${contextLabel(contextId)}. Showing the previous view.`,
              () => void viewportQuery.refetch()
            )
          );
        }
        const hasLoadingOrError = children.some(
          item =>
            item.nvtx?.kind === 'status' &&
            (item.nvtx.state === 'loading' || item.nvtx.state === 'error')
        );
        if (renderedItems === 0 && !hasLoadingOrError) {
          children.push(
            statusItem(
              `nvtx:${contextId}:empty`,
              'empty',
              'Adjust the time window or filters to see captured NVTX activity.'
            )
          );
        }
        placeGroup(contextId, children);
      }
    });

    const controls =
      catalogControls.length > 0 ? (
        <NvtxTimelineControls
          catalogs={catalogControls}
          selections={selections}
          statistics={statisticsControls}
          onSelectionChange={setContextSelection}
        />
      ) : null;
    return { placements, initiallyExpandedIds, controls };
  }, [
    catalogQueries,
    contextIds,
    contextResources,
    contextsQuery,
    resourceIds,
    rootResourceId,
    selections,
    setContextSelection,
    viewportQueries,
  ]);
}
