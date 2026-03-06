import { ResourceGroup } from '~quent/types/ResourceGroup';
import { ResourceTypeSelector } from './ResourceTypeSelector';

interface ResourceGroupRowProps {
  group: ResourceGroup;
  id: string;
  availableResourceTypes?: string[];
  selectedType?: string;
  onTypeChange?: (itemId: string, type: string) => void;
  verbose?: boolean;
  compact?: boolean;
}

export const ResourceGroupRow = ({
  group,
  id,
  availableResourceTypes,
  selectedType,
  onTypeChange,
  compact,
}: ResourceGroupRowProps): React.ReactNode => {
  const hasMultipleChildTypes = (availableResourceTypes?.length ?? 0) > 1;

  const selector = hasMultipleChildTypes &&
    selectedType &&
    onTypeChange &&
    availableResourceTypes && (
      <ResourceTypeSelector
        id={id}
        selectedType={selectedType}
        availableResourceTypes={availableResourceTypes}
        onTypeChange={onTypeChange}
        compact={compact}
        className={compact ? 'ml-2' : 'mt-1'}
      />
    );

  if (compact) {
    return (
      <div className="flex items-center">
        <span className="text-xs font-bold">{group.instance_name}</span>
        {selector}
      </div>
    );
  }

  return (
    <div>
      <div>
        <span className="text-xs font-bold">{group.instance_name}</span>
      </div>
      <div className="text-xs text-muted-foreground">{group.id}</div>
      {selector}
    </div>
  );
};
