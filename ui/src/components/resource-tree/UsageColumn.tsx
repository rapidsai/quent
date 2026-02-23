import { useAtomValue, useSetAtom } from 'jotai';
import { EntityTypeKey } from '@/types';
import { QueryBundle } from '~quent/types/QueryBundle';
import { TreeTableItem } from './types';
import { ResourceTimeline } from '../timeline/ResourceTimeline';
import {
  timelineDataAtom,
  xAxisRangeAtom,
  isTimelineHoveredAtom,
  hoveredTimelineIdAtom,
} from '@/atoms/timeline';

type UsageColumnProps = {
  item: TreeTableItem;
  engineId: string;
  queryBundle: QueryBundle;
  selectedTypes: Map<string, string>;
  startTime: bigint;
  durationSeconds: number;
};

export function UsageColumn({
  item,
  engineId,
  queryBundle,
  selectedTypes,
  startTime,
  durationSeconds,
}: UsageColumnProps): React.ReactNode {
  const timelineData = useAtomValue(timelineDataAtom(item.id));
  const xAxisRange = useAtomValue(xAxisRangeAtom);
  const isHovered = useAtomValue(isTimelineHoveredAtom(item.id));
  const setHoveredId = useSetAtom(hoveredTimelineIdAtom);

  const entity = item?.entity ?? {};
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
        preloadedData={timelineData}
        xAxisRange={xAxisRange}
      />
    </div>
  );
}
