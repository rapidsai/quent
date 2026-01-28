import { Column, TreeTable } from '@/components/ui/tree-table';
import { useMemo } from 'react';
import { ResourceTree } from '~quent/types/ResourceTree';
import { ResourceTimeline } from './timeline/ResourceTimeline';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { EntitiesUI } from '~quent/types/EntitiesUI';
import { EntityTypeValue, EntityRefKey } from '@/types';
import { getIconForType, getRowForEntity, TreeTableItem } from './resource-tree/ResourceTreeRow';
import { QueryBundle } from '~quent/types/QueryBundle';

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle;
}

export function QueryResourceTree({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const treeData = useMemo(() => {
    const transformResourceTree = (resourceTree: ResourceTree): TreeTableItem => {
      const [entityType, entityId] = resourceTree.item
        ? Object.entries(resourceTree.item)[0]
        : ['Root' as EntityRefKey, 'root' as string];

      const entityKey = entityRefToEntitiesKey(entityType as EntityRefKey) as keyof EntitiesUI;
      // Special case for engine, there can only and will always be one
      const entity: EntityTypeValue | undefined =
        entityKey === 'engine' ? entities.engine : entities[entityKey]?.[entityId];

      return {
        id: entityId,
        type: entityType,
        entity: entity as EntityTypeValue,
        icon: getIconForType(entityType),
        children: resourceTree.children?.map(child => transformResourceTree(child)) ?? null,
      };
    };
    return [transformResourceTree(resourceTree)];
  }, [resourceTree, entities]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({ item }: { item: TreeTableItem; level: number }) => (
          <div className="text-foreground flex items-center py-2">
            <div>{item.icon && <item.icon className="h-4 w-4 shrink-0 mr-4" />}</div>
            <div>{getRowForEntity(item)}</div>
          </div>
        ),
      },
      {
        key: 'usage',
        label: 'Usage',
        widthIndex: 1,
        render: ({ item }: { item: TreeTableItem }) => {
          const entity = item?.entity ?? {};
          const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
          const fsmTypeName =
            entityTypeName && queryBundle.entities.resources_types[entityTypeName]?.used_by_fsms[0];
          return item.type === 'Resource' ? (
            <ResourceTimeline
              engineId={engineId}
              queryId={queryBundle.query_id}
              resourceId={item.id}
              startTime={queryBundle.start_time_unix_ns}
              durationSeconds={queryBundle.duration_s}
              fsmTypeName={fsmTypeName ?? undefined}
            />
          ) : (
            // TODO: Aggregate all of the children into an aggregate timeline
            // <Timeline timestamps={[]} series={{}} />
            <div className="h-full items-center flex"> -- </div>
          );
        },
      },
    ] satisfies Column<TreeTableItem>[];
  }, [engineId, queryBundle]);

  return (
    <div className="space-y-4 w-full h-full">
      <TreeTable<TreeTableItem>
        data={treeData}
        columns={columns}
        initialSelectedItemId={'root'}
        columnWidths={[350, 500]}
      />
    </div>
  );
}
