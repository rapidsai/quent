import { EntityTypeKey } from '@/types';
import { TreeTableItem } from '@/components/resource-tree/types';
import { Database, Folder, LineChart, LucideIcon, Network, Rocket, Star } from 'lucide-react';

/**
 * Recursively collect all unique resource type names from a tree of TreeTableItems.
 */
export function collectResourceTypesFromTree(items: TreeTableItem[]): string[] {
  const types = new Set<string>();

  const collect = (items: TreeTableItem[]) => {
    items.forEach(item => {
      if ('type_name' in item.entity && !item.children?.length) {
        const typeName = item.entity.type_name;
        if (typeName) types.add(typeName);
      }
      if (item.children?.length) collect(item.children);
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
      return Star;
  }
}
