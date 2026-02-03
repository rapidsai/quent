import { Column, TreeTable } from '@/components/ui/tree-table';
import { useMemo, useState } from 'react';
import { ResourceTree } from '~quent/types/ResourceTree';
import { ResourceTimeline } from './timeline/ResourceTimeline';
import { entityRefToEntitiesKey } from '@/lib/queryBundle.utils';
import { collectResourceTypesFromTree, getIconForType } from '@/lib/resource.utils';
import { EntitiesUI } from '~quent/types/EntitiesUI';
import { EntityTypeValue, EntityRefKey, EntityTypeKey } from '@/types';
import { TreeTableItem, ResourceGroupRow, ResourceRow } from './resource-tree/ResourceTreeRows';
import { QueryBundle } from '~quent/types/QueryBundle';
import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Resource } from '~quent/types/Resource';

interface QueryResourceTreeProps {
  engineId: string;
  queryBundle: QueryBundle;
}

export function QueryResourceTree({ queryBundle, engineId }: QueryResourceTreeProps) {
  const { entities, resource_tree: resourceTree } = queryBundle;
  const [selectedTypes, setSelectedTypes] = useState<Map<string, string>>(new Map());

  const treeData = useMemo(() => {
    const transformResourceTree = (resourceTree: ResourceTree): TreeTableItem => {
      const [entityType, entityId] = resourceTree.item
        ? Object.entries(resourceTree.item)[0]
        : ['Root' as EntityRefKey, 'root' as string];

      const entityKey = entityRefToEntitiesKey(entityType as EntityRefKey) as keyof EntitiesUI;
      // Special case for engine, there can only and will always be one
      const entity: EntityTypeValue | undefined =
        entityKey === 'engine' ? entities.engine : entities[entityKey]?.[entityId];

      let iconKey = entityType ?? 'Resource';
      if (entityType === EntityTypeKey.ResourceGroup && entity) {
        if ('type_name' in entity && entity.type_name) {
          iconKey = entity.type_name as string;
        } else if ('instance_name' in entity && entity.instance_name) {
          iconKey = entity.instance_name as string;
        }
      }

      const children = resourceTree.children?.map(child => transformResourceTree(child)) ?? null;

      // For ResourceGroups, collect available resource types from descendants
      let availableResourceTypes: string[] | undefined;
      if (entityType === EntityTypeKey.ResourceGroup && children) {
        availableResourceTypes = collectResourceTypesFromTree(children);
      }

      return {
        id: entityId,
        type: entityType,
        entity: entity as EntityTypeValue,
        icon: getIconForType(iconKey),
        children,
        availableResourceTypes,
      };
    };
    const rootNode = transformResourceTree(resourceTree);
    return [{ ...rootNode, expanded: true }];
  }, [resourceTree, entities]);

  const columns = useMemo(() => {
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
                {item.type === EntityTypeKey.ResourceGroup ? (
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
        render: ({ item }: { item: TreeTableItem }) => {
          const entity = item?.entity ?? {};
          const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
          const fsmTypeName =
            entityTypeName && queryBundle.entities.resources_types[entityTypeName]?.used_by_fsms[0];
          if (item.type === EntityTypeKey.Resource) {
            return (
              <ResourceTimeline
                engineId={engineId}
                queryId={queryBundle.query_id}
                resourceId={item.id}
                resourceType={item.type}
                startTime={queryBundle.start_time_unix_ns}
                durationSeconds={queryBundle.duration_s}
                fsmTypeName={fsmTypeName ?? undefined}
              />
            );
          } else if (item.type === EntityTypeKey.ResourceGroup) {
            const instanceName =
              'instance_name' in entity ? (entity.instance_name as string) : undefined;
            const selectedType =
              selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';

            if (!selectedType) {
              return <div className="h-full items-center flex">No resources</div>;
            }

            return (
              <ResourceTimeline
                engineId={engineId}
                queryId={queryBundle.query_id}
                resourceId={item.id}
                resourceType={EntityTypeKey.ResourceGroup}
                startTime={queryBundle.start_time_unix_ns}
                durationSeconds={queryBundle.duration_s}
                fsmTypeName={undefined}
                resourceTypeName={selectedType}
                instanceName={instanceName}
              />
            );
          } else {
            return (
              // TODO: Aggregate all of the children into an aggregate timeline
              // <Timeline timestamps={[]} series={{}} />
              <div className="h-full items-center flex"> -- </div>
            );
          }
        },
      },
    ] satisfies Column<TreeTableItem>[];
  }, [engineId, queryBundle, selectedTypes]);

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
