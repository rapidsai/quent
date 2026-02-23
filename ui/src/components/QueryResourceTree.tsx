import { Column, TreeTable } from '@/components/ui/tree-table';
import { useCallback, useMemo, useState } from 'react';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { Provider } from 'jotai';
import { useHydrateAtoms } from 'jotai/utils';
import { ResourceTree } from '~quent/types/ResourceTree';
import { TimelineController } from './timeline/TimelineController';
import { collectResourceTypesFromTree } from '@/lib/resource.utils';
import { EntityRefKey } from '@/types';
import { TreeTableItem } from './resource-tree/types';
import { ResourceColumn } from './resource-tree/ResourceColumn';
import { UsageColumn } from './resource-tree/UsageColumn';
import { QueryBundle } from '~quent/types/QueryBundle';
import { fetchResourceGroupTimeline, DEFAULT_STALE_TIME } from '@/services/api';
import { transformResourceTree, getAdaptiveNumBins } from '@/lib/timeline.utils';
import { useExpandedIds, useBulkTimelines } from '@/hooks/useBulkTimelines';
import { zoomRangeAtom, debouncedZoomRangeAtom, startTimeMsAtom } from '@/atoms/timeline';

function getRootResourceGroupId(resourceTree: ResourceTree): string | null {
  if (!('ResourceGroup' in resourceTree)) return null;
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle;
}

export function QueryResourceTree(props: QueryResourceTreeProps) {
  return (
    <Provider>
      <QueryResourceTreeContent {...props} />
    </Provider>
  );
}

function QueryResourceTreeContent({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useState<Map<string, string>>(new Map());

  const startTime = queryBundle.start_time_unix_ns;
  const durationSeconds = queryBundle.duration_s;
  const startTimeMs = useMemo(() => Number(startTime / 1_000_000n), [startTime]);

  useHydrateAtoms([
    [zoomRangeAtom, { start: 0, end: durationSeconds }],
    [debouncedZoomRangeAtom, { start: 0, end: durationSeconds }],
    [startTimeMsAtom, startTimeMs],
  ]);

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useState<string>(resourceTypeOptions[0] || '');

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  const { expandedIds, handleExpandChange } = useExpandedIds(rootItem.id);

  const { handleZoomChange, handleExpand, invalidateItem } = useBulkTimelines({
    engineId,
    queryId: queryBundle.query_id,
    rootItem,
    expandedIds,
    selectedTypes,
    entities,
  });

  const onExpandChange = useCallback(
    (itemId: string, isExpanded: boolean) => {
      handleExpandChange(itemId, isExpanded);
      handleExpand(itemId, isExpanded);
    },
    [handleExpandChange, handleExpand]
  );

  const { data: rootTimelineData } = useQuery({
    queryKey: [
      'resourceGroupTimeline',
      'root',
      engineId,
      queryBundle.query_id,
      rootResourceGroupId,
      durationSeconds,
      rootResourceType,
    ],
    queryFn: () =>
      fetchResourceGroupTimeline(engineId, queryBundle.query_id, rootResourceGroupId!, {
        num_bins: getAdaptiveNumBins(durationSeconds),
        start: 0,
        end: durationSeconds,
        duration: durationSeconds,
        resource_type_name: rootResourceType,
      }),
    staleTime: DEFAULT_STALE_TIME,
    enabled: rootResourceGroupId != null && !!rootResourceType,
    placeholderData: keepPreviousData,
  });

  const treeData = useMemo(() => [rootItem], [rootItem]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({ item }: { item: TreeTableItem; level: number }) => (
          <ResourceColumn
            item={item}
            selectedType={selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || ''}
            onTypeChange={(itemId, newType) => {
              setSelectedTypes(prev => new Map(prev).set(itemId, newType));
              invalidateItem(itemId);
              if (itemId === rootItem.id) {
                setRootResourceType(newType);
              }
            }}
          />
        ),
      },
      {
        key: 'usage',
        label: 'Usage',
        widthIndex: 1,
        subHeaderContent: (
          <div className="h-full overflow-hidden flex items-center py-2">
            <TimelineController
              startTime={startTime}
              durationSeconds={durationSeconds}
              timelineData={rootTimelineData}
              onZoomChange={handleZoomChange}
            />
          </div>
        ),
        render: ({ item }: { item: TreeTableItem }) => (
          <UsageColumn
            item={item}
            engineId={engineId}
            queryBundle={queryBundle}
            selectedTypes={selectedTypes}
            startTime={startTime}
            durationSeconds={durationSeconds}
          />
        ),
      },
    ] satisfies Column<TreeTableItem>[];
  }, [
    startTime,
    durationSeconds,
    rootTimelineData,
    selectedTypes,
    rootItem,
    engineId,
    queryBundle,
    handleZoomChange,
    invalidateItem,
  ]);

  return (
    <TreeTable<TreeTableItem>
      data={treeData}
      columns={columns}
      initialSelectedItemId={rootItem.id}
      columnWidths={[275, 'auto']}
      onExpandChange={onExpandChange}
    />
  );
}
