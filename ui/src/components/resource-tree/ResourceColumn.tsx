// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Resource } from '~quent/types/Resource';
import { useAtomValue } from 'jotai';
import { cn } from '@/lib/utils';
import { TreeTableItem } from './types';
import { ResourceGroupRow } from './ResourceGroupRow';
import { ResourceRow } from './ResourceRow';
import { timelineDensityAtom } from '@/atoms/timeline';

type ResourceColumnProps = {
  item: TreeTableItem;
  selectedType: string;
  onTypeChange: (itemId: string, type: string) => void;
  availableFsmTypes?: string[];
  selectedFsmType?: string | null;
  onFsmChange?: (itemId: string, fsmType: string | null) => void;
  className?: string;
  verbose?: boolean;
};

export function ResourceColumn({
  item,
  selectedType,
  onTypeChange,
  availableFsmTypes,
  selectedFsmType,
  onFsmChange,
  className,
}: ResourceColumnProps): React.ReactNode {
  const compact = useAtomValue(timelineDensityAtom) === 'compact';

  return (
    <div
      className={cn(
        'text-foreground flex truncate items-center',
        compact ? 'py-0' : 'py-2',
        className
      )}
    >
      <div>{item.icon && <item.icon className="h-4 w-4 shrink-0 mr-4" />}</div>
      <div>
        {item?.children?.length ? (
          <ResourceGroupRow
            group={item.entity as ResourceGroup}
            id={item.id}
            availableResourceTypes={item.availableResourceTypes}
            selectedType={selectedType}
            onTypeChange={onTypeChange}
            compact={compact}
            availableFsmTypes={availableFsmTypes}
            selectedFsmType={selectedFsmType}
            onFsmChange={onFsmChange}
          />
        ) : (
          <ResourceRow resource={item.entity as Resource} compact={compact} />
        )}
      </div>
    </div>
  );
}
