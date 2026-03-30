// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { EntityTypeKey } from '@/types';
import { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import { TreeTableItem } from './types';
import { ResourceTimeline } from '../timeline/ResourceTimeline';
import { isTimelineHoveredAtom, hoveredTimelineIdAtom } from '@/atoms/timeline';

type UsageColumnProps = {
  item: TreeTableItem;
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  selectedTypes: Map<string, string>;
  selectedFsmTypes?: Map<string, string | null>;
  startTime: bigint;
  durationSeconds: number;
};

export function UsageColumn({
  item,
  engineId,
  queryBundle,
  selectedTypes,
  selectedFsmTypes,
  startTime,
  durationSeconds,
}: UsageColumnProps): React.ReactNode {
  const isHovered = useAtomValue(isTimelineHoveredAtom(item.id));
  const setHoveredId = useSetAtom(hoveredTimelineIdAtom);

  const entity = item?.entity ?? {};
  const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
  const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
  const resourceType =
    item.type === EntityTypeKey.Resource ? EntityTypeKey.Resource : EntityTypeKey.ResourceGroup;
  const resourceTypeName =
    resourceType === EntityTypeKey.ResourceGroup ? selectedType : entityTypeName;
  const resourceTypeDecl = resourceTypeName
    ? queryBundle.entities.resource_types[resourceTypeName]
    : undefined;
  const usedBy = resourceTypeDecl?.used_by;
  let fsmTypeName: string | undefined;
  if (usedBy?.length === 1) {
    fsmTypeName = usedBy[0];
  } else if (resourceType === EntityTypeKey.ResourceGroup) {
    fsmTypeName = selectedFsmTypes?.get(item.id) ?? undefined;
  }
  const capacities = resourceTypeDecl?.capacities;
  return (
    <div
      onMouseEnter={() => setHoveredId(item.id)}
      onMouseLeave={() => setHoveredId(null)}
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
        showTooltip={isHovered}
        capacities={capacities}
        quantitySpecs={queryBundle.quantity_specs}
      />
    </div>
  );
}
