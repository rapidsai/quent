import { Column, TreeTable } from '@/components/ui/tree-table';
import { useMemo } from 'react';
import { ResourceTree } from '~quent/types/ResourceTree';
import { ResourceTimeline } from './timeline/ResourceTimeline';
import { Database, Folder, LineChart, LucideIcon, Rocket } from 'lucide-react';
import { cn } from '@/lib/utils';

interface NodeProfileProps {
  engineId: string;
  resourceTree: ResourceTree;
}

type TreeTableItem = {
  id: string;
  type: string;
  icon: LucideIcon;
  children?: TreeTableItem[];
};

const getIconForType = (type: string): LucideIcon => {
  switch (type) {
    case 'Engine':
      return Database;
    case 'QueryGroup':
    case 'ResourceGroup':
      return Folder;
    case 'Worker':
      return Rocket;
    case 'Resource':
      return LineChart;
    default:
      return Database;
  }
};

export function NodeProfile({ resourceTree, engineId }: NodeProfileProps) {
  const treeData = useMemo(() => {
    const transformResourceTree = (resourceTree: ResourceTree): TreeTableItem => {
      const [entityType, entityId] = resourceTree.item
        ? Object.entries(resourceTree.item)[0]
        : ['Root', 'root'];
      return {
        id: entityId,
        type: entityType,
        icon: getIconForType(entityType),
        children: resourceTree.children?.map(child => transformResourceTree(child)) ?? null,
      };
    };
    return [transformResourceTree(resourceTree)];
  }, [resourceTree]);

  const columns = useMemo(() => {
    return [
      {
        key: 'resource',
        label: 'Resource',
        widthIndex: 0,
        isFirst: true,
        render: ({
          item,
          isSelected,
        }: {
          item: TreeTableItem;
          level: number;
          isSelected: boolean;
        }) => (
          <span
            className={cn(
              {
                'text-foreground': isSelected,
                'text-foreground-muted': !isSelected,
              },
              'flex items-center'
            )}
          >
            {item.icon && <item.icon className="h-4 w-4 shrink-0 mr-2" />}
            <span className="font-extrabold mr-1">{item.type}</span>({item.id})
          </span>
        ),
      },
      {
        key: 'usage',
        label: 'Usage',
        widthIndex: 1,
        render: ({ item }: { item: TreeTableItem }) => {
          return item.type === 'Resource' ? (
            <ResourceTimeline engineId={engineId} resourceId={item.id} />
          ) : (
            // TODO: Aggregate all of the children into an aggregate timeline
            // <Timeline timestamps={[]} series={{}} />
            <div> -- </div>
          );
        },
      },
    ] satisfies Column<TreeTableItem>[];
  }, [engineId]);

  return (
    <div className="space-y-4 w-full h-full">
      <TreeTable<TreeTableItem> data={treeData} columns={columns} initialSelectedItemId={'root'} />
    </div>
  );
}
