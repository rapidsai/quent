import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Resource } from '~quent/types/Resource';
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

interface ResourceGroupRowProps {
  group: ResourceGroup;
  id: string;
  availableResourceTypes?: string[];
  selectedType?: string;
  onTypeChange?: (itemId: string, type: string) => void;
}

export const ResourceGroupRow = ({
  group,
  id,
  availableResourceTypes,
  selectedType,
  onTypeChange,
}: ResourceGroupRowProps): React.ReactNode => {
  const hasMultipleChildTypes = (availableResourceTypes?.length ?? 0) > 1;

  return (
    <div>
      <div>
        <span className="text-xs font-bold">{group.instance_name}</span>
      </div>
      <div className="text-xs text-muted-foreground">{group.id}</div>
      {hasMultipleChildTypes && selectedType && onTypeChange && (
        <div
          className="flex items-center gap-2 mt-1"
          onClick={e => e.stopPropagation()}
          onMouseDown={e => e.stopPropagation()}
        >
          <label htmlFor={`type-select-${id}`} className="text-xs text-muted-foreground">
            Type:
          </label>
          <select
            id={`type-select-${id}`}
            value={selectedType}
            onChange={e => {
              e.stopPropagation();
              onTypeChange?.(id, e.target.value);
            }}
            className="text-xs bg-background border border-border rounded px-1 py-0.5 text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          >
            {availableResourceTypes!.map(typeOption => (
              <option key={typeOption} value={typeOption}>
                {typeOption}
              </option>
            ))}
          </select>
        </div>
      )}
    </div>
  );
};

interface ResourceRowProps {
  resource: Resource;
}

export const ResourceRow = ({ resource }: ResourceRowProps): React.ReactNode => {
  return (
    <div>
      <div>
        <span className="text-xs font-bold">
          {resource.instance_name}{' '}
          {resource.type_name !== resource.instance_name && resource.type_name
            ? `(${resource.type_name})`
            : ''}
        </span>
      </div>
      <div className="text-xs text-muted-foreground">{resource.id}</div>
    </div>
  );
};
