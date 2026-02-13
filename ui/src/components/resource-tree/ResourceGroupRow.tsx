import { ResourceGroup } from '~quent/types/ResourceGroup';
import { ResourceTypeSelector } from './ResourceTypeSelector';

interface ResourceGroupRowProps {
  group: ResourceGroup;
  id: string;
  availableResourceTypes?: string[];
  selectedType?: string;
  onTypeChange?: (itemId: string, type: string) => void;
  verbose?: boolean;
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
      {hasMultipleChildTypes && selectedType && onTypeChange && availableResourceTypes && (
        <ResourceTypeSelector
          id={id}
          selectedType={selectedType}
          availableResourceTypes={availableResourceTypes}
          onTypeChange={onTypeChange}
          className="mt-1"
        />
      )}
    </div>
  );
};
