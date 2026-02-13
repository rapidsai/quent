import { Column, TreeTable } from '@/components/ui/tree-table';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { ResourceTree } from '~quent/types/ResourceTree';
import { TimelineController } from './timeline/TimelineController';
import { collectResourceTypesFromTree } from '@/lib/resource.utils';
import { EntityRefKey } from '@/types';
import { TreeTableItem } from './resource-tree/types';
import { ResourceColumn } from './resource-tree/ResourceColumn';
import { UsageColumn } from './resource-tree/UsageColumn';
import { QueryBundle } from '~quent/types/QueryBundle';
import { fetchResourceGroupTimeline, DEFAULT_STALE_TIME } from '@/services/api';
import { transformResourceTree } from '@/lib/timeline.utils';

// Number of bins used by all timelines - must match ResourceTimeline
const NUM_TIMELINE_BINS = 200;

/** Root resource group id when resource_tree root is a ResourceGroup */
function getRootResourceGroupId(resourceTree: ResourceTree): string | null {
  if (!('ResourceGroup' in resourceTree)) return null;
  const [, entityId] = Object.entries(resourceTree.ResourceGroup.id)[0] as [EntityRefKey, string];
  return entityId;
}

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle;
}

export function QueryResourceTree({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useState<Map<string, string>>(new Map());

  const rootItem = useMemo(
    () => transformResourceTree(entities, resourceTree),
    [resourceTree, entities]
  );

  const resourceTypeOptions = useMemo(() => collectResourceTypesFromTree([rootItem]), [rootItem]);

  const [rootResourceType, setRootResourceType] = useState<string>(resourceTypeOptions[0] || '');
  const [hoveredTimelineId, setHoveredTimelineId] = useState<string | null>(null);

  const rootResourceGroupId = useMemo(() => getRootResourceGroupId(resourceTree), [resourceTree]);

  const startTime = queryBundle.start_time_unix_ns;
  const durationSeconds = queryBundle.duration_s;

  const resourceTypeName = 'memory';
  const { data: rootTimelineData } = useQuery({
    queryKey: [
      'resourceGroupTimeline',
      'root',
      engineId,
      queryBundle.query_id,
      rootResourceGroupId,
      resourceTypeName,
      durationSeconds,
      rootResourceType,
    ],
    queryFn: () =>
      fetchResourceGroupTimeline(engineId, queryBundle.query_id, rootResourceGroupId!, {
        num_bins: NUM_TIMELINE_BINS,
        start: 0,
        end: durationSeconds,
        resource_type_name: rootResourceType,
      }),
    staleTime: DEFAULT_STALE_TIME,
    enabled: rootResourceGroupId != null && !!rootResourceType,
  });

  // Little funky but the tree expects an array of items, so to avoid creating
  // a new array each render we'll memoize
  const treeData = useMemo(() => {
    return [rootItem];
  }, [rootItem]);

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
              // Update timeline controller with the same type selected in the root resource
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
              numBins={NUM_TIMELINE_BINS}
              timelineData={rootTimelineData}
            />
          </div>
        ),
        render: ({ item }: { item: TreeTableItem }) => (
          <UsageColumn
            item={item}
            engineId={engineId}
            queryBundle={queryBundle}
            selectedTypes={selectedTypes}
            hoveredTimelineId={hoveredTimelineId}
            setHoveredTimelineId={setHoveredTimelineId}
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
    hoveredTimelineId,
  ]);

  return (
    <TreeTable<TreeTableItem>
      data={treeData}
      columns={columns}
      initialSelectedItemId={rootItem.id}
      columnWidths={[275, 'auto']}
    />
  );
}
