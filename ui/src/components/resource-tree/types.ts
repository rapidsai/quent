import { EntityTypeValue } from '@/types';
import { LucideIcon } from 'lucide-react';

export type TreeTableItem = {
  id: string;
  type: string;
  entity: EntityTypeValue;
  icon: LucideIcon;
  children?: TreeTableItem[];
  availableResourceTypes?: string[];
};
