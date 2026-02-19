import { EntityTypeKey } from '@/types';
import { QueryBundle } from '~quent/types/QueryBundle';
import { TreeTableItem } from './types';
import { ResourceTimeline } from '../timeline/ResourceTimeline';

type UsageColumnProps = {
  item: TreeTableItem;
  engineId: string;
  queryBundle: QueryBundle;
  selectedTypes: Map<string, string>;
  hoveredTimelineId: string | null;
  setHoveredTimelineId: React.Dispatch<React.SetStateAction<string | null>>;
  startTime: bigint;
  durationSeconds: number;
  zoomState: { startPct: number; endPct: number };
};

export function UsageColumn({
  item,
  engineId,
  queryBundle,
  selectedTypes,
  hoveredTimelineId,
  setHoveredTimelineId,
  startTime,
  durationSeconds,
  zoomState,
}: UsageColumnProps): React.ReactNode {
  const entity = item?.entity ?? {};
  // Look up FSM type name from the resource type's used_by field
  const entityTypeName = 'type_name' in entity ? (entity.type_name as string) : undefined;
  const usedBy = entityTypeName
    ? queryBundle.entities.resource_types[entityTypeName]?.used_by
    : undefined;
  const fsmTypeName = usedBy?.[0];
  const selectedType = selectedTypes.get(item.id) || item.availableResourceTypes?.[0] || '';
  const resourceType =
    item.type === EntityTypeKey.Resource ? EntityTypeKey.Resource : EntityTypeKey.ResourceGroup;

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
        showTooltip={hoveredTimelineId === item.id}
        zoomState={zoomState}
      />
    </div>
  );
}
