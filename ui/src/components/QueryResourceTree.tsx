import { Column, TreeTable } from '@/components/ui/tree-table';
import { useMemo, useState } from 'react';
import { ResourceTree } from '~quent/types/ResourceTree';
import { ResourceTimeline } from './timeline/ResourceTimeline';
import { TimelineController } from './timeline/TimelineController';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { collectResourceTypesFromTree, getIconForType } from '@/lib/resource.utils';
import { QueryEntities } from '~quent/types/QueryEntities';
import { EntityTypeValue, EntityRefKey, EntityTypeKey } from '@/types';
import { TreeTableItem, ResourceGroupRow, ResourceRow } from './resource-tree/ResourceTreeRows';
import { QueryBundle } from '~quent/types/QueryBundle';
import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Resource } from '~quent/types/Resource';

// Number of bins used by all timelines - must match ResourceTimeline
const NUM_TIMELINE_BINS = 100;

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle;
}

// Helper function to lookup entity from QueryEntities
const lookupEntity = (
  entities: QueryEntities,
  entityType: EntityRefKey,
  entityId: string
): EntityTypeValue | undefined => {
  const entityKey = entityRefToEntitiesKey(entityType) as keyof QueryEntities;
  const entityValue = entities[entityKey];

  // SingleEntity (Engine | Query | QueryGroup): single object with id
  if ('id' in entityValue && entityValue.id === entityId) {
    return entityValue as EntityTypeValue;
  }
  // Record<string, EntityTypeValue>: lookup by entityId key
  return (entityValue as Record<string, EntityTypeValue>)?.[entityId];
};

const transformResourceTree = (
  entities: QueryEntities,
  resourceTree: ResourceTree
): TreeTableItem => {
  if ('ResourceGroup' in resourceTree) {
    const node = resourceTree.ResourceGroup;
    const [entityType, entityId] = Object.entries(node.id)[0] as [EntityRefKey, string];
    const entity = lookupEntity(entities, entityType, entityId);
    const children = node.children.map(child => transformResourceTree(entities, child));
    const availableResourceTypes = collectResourceTypesFromTree(children);

    return {
      id: entityId,
      type: entityType,
      entity: entity as EntityTypeValue,
      icon: getIconForType(entityType),
      children,
      availableResourceTypes,
    };
  } else {
    const [entityType, entityId] = Object.entries(resourceTree.Resource)[0] as [
      EntityRefKey,
      string,
    ];
    const entity = lookupEntity(entities, entityType, entityId);

    return {
      id: entityId,
      type: entityType,
      entity: entity as EntityTypeValue,
      icon: getIconForType(entityType),
      children: [],
      availableResourceTypes: undefined,
    };
  }
};

export function QueryResourceTree({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useState<Map<string, string>>(new Map());
  const [hoveredTimelineId, setHoveredTimelineId] = useState<string | null>(null);

  const treeData = useMemo(() => {
    const rootNode = transformResourceTree(entities, resourceTree);

    // Skip the root node (engine) and return its children directly
    return rootNode.children || [];
  }, [resourceTree, entities]);

  const columns = useMemo(() => {
    const startTime = queryBundle.start_time_unix_ns;
    const durationSeconds = queryBundle.duration_s;

    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({ item }: { item: TreeTableItem; level: number }) => {
          const handleTypeChange = (itemId: string, newType: string) => {
            setSelectedTypes(prev => new Map(prev).set(itemId, newType));
          };
          const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
          return (
            <div className="text-foreground flex items-center py-2">
              <div>{item.icon && <item.icon className="h-4 w-4 shrink-0 mr-4" />}</div>
              <div>
                {item?.children?.length ? (
                  <ResourceGroupRow
                    group={item.entity as ResourceGroup}
                    id={item.id}
                    availableResourceTypes={item.availableResourceTypes}
                    selectedType={selectedType}
                    onTypeChange={handleTypeChange}
                  />
                ) : (
                  <ResourceRow resource={item.entity as Resource} />
                )}
              </div>
            </div>
          );
        },
      },
      {
        key: 'usage',
        label: 'Usage',
        widthIndex: 1,
        subHeaderContent: (
          <TimelineController
            startTime={queryBundle.start_time_unix_ns}
            durationSeconds={queryBundle.duration_s}
            numBins={NUM_TIMELINE_BINS}
          />
        ),
        render: ({ item }: { item: TreeTableItem }) => {
          const entity = item?.entity ?? {};
          // Look up FSM type name from the resource type's used_by field
          const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
          const usedBy = entityTypeName
            ? queryBundle.entities.resource_types[entityTypeName]?.used_by
            : undefined;
          const fsmTypeName = usedBy && usedBy?.length > 0 ? usedBy[0] : undefined;
          const instanceName =
            'instance_name' in entity ? (entity.instance_name as string) : undefined;
          const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
          const resourceType =
            item.type === EntityTypeKey.Resource
              ? EntityTypeKey.Resource
              : EntityTypeKey.ResourceGroup;

          return (
            <div
              onMouseEnter={() => setHoveredTimelineId(item.id)}
              onMouseLeave={() => setHoveredTimelineId(null)}
              onClick={e => e.stopPropagation()}
              className="h-full w-full"
            >
              <ResourceTimeline
                engineId={engineId}
                queryId={queryBundle.query_id}
                resourceId={item.id}
                resourceType={resourceType}
                startTime={startTime}
                durationSeconds={durationSeconds}
                fsmTypeName={fsmTypeName}
                resourceTypeName={selectedType}
                instanceName={instanceName}
                showTooltip={hoveredTimelineId === item.id}
              />
            </div>
          );
        },
      },
    ] satisfies Column<TreeTableItem>[];
  }, [engineId, queryBundle, selectedTypes, hoveredTimelineId]);

  return (
    <TreeTable<TreeTableItem>
      data={treeData}
      columns={columns}
      initialSelectedItemId={'root'}
      columnWidths={[275, 'auto']}
    />
  );
}
