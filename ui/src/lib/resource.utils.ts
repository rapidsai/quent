import { EntityTypeKey } from '@/types';
import { TreeTableItem } from '@/components/resource-tree/ResourceTreeRows';
import { Database, Folder, LineChart, LucideIcon, Network, Rocket } from 'lucide-react';

/**
 * Recursively collect all unique resource type names from a tree of TreeTableItems.
 */
export function collectResourceTypesFromTree(items: TreeTableItem[]): string[] {
  const types = new Set<string>();

  const collect = (items: TreeTableItem[]) => {
    items.forEach(item => {
      if (item.type === EntityTypeKey.Resource && 'type_name' in item.entity) {
        const typeName = (item.entity as { type_name?: string }).type_name;
        if (typeName) types.add(typeName);
      }
      if (item.children) collect(item.children);
    });
  };

  collect(items);
  return Array.from(types);
}

export function getIconForType(typeOrInstanceName: string): LucideIcon {
  switch (typeOrInstanceName) {
    // Entity types
    case EntityTypeKey.Resource:
      return LineChart;
    case EntityTypeKey.ResourceGroup:
      return Folder;
    // ResourceGroup type_names (for more specific icons)
    case 'Engine':
      return Database;
    case 'Network':
      return Network;
    case 'Worker':
      return Rocket;
    default:
      return Database;
  }
}
