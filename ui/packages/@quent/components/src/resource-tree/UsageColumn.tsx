// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { EntityTypeKey } from '@quent/utils';
import { QueryBundle } from '@quent/utils';
import { DEFAULT_TIMELINE_HEIGHT } from '../timeline/types';
import type { EntityRef } from '@quent/utils';
import { TreeTableItem } from './types';
import { ResourceTimeline } from '../timeline/ResourceTimeline';

type UsageColumnProps = {
  item: TreeTableItem;
  engineId: string;
  queryBundle: QueryBundle<EntityRef>;
  selectedTypes: Map<string, string>;
  selectedFsmTypes?: Map<string, string | null>;
  startTime: bigint;
  durationSeconds: number;
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
};

/** Table column cell that renders a per-resource timeline. */
export function UsageColumn({
  item,
  engineId,
  queryBundle,
  selectedTypes,
  selectedFsmTypes,
  startTime,
  durationSeconds,
  isDark,
}: UsageColumnProps): React.ReactNode {
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
  // Cell wrapper kept (without enter/leave) so click events still don't
  // propagate to the table-row click handler. Tooltip visibility is driven
  // by the chart's own pointermove via `timelineHoverAtom` — no row-level
  // gating needed.
  return (
    <div
      onClick={e => e.stopPropagation()}
      className="h-full w-full"
      style={{ minHeight: DEFAULT_TIMELINE_HEIGHT }}
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
        capacities={capacities}
        quantitySpecs={queryBundle.quantity_specs}
        fsmTypes={queryBundle.entities.fsm_types}
        isDark={isDark}
      />
    </div>
  );
}
