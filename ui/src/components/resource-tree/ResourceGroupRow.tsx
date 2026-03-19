import { ResourceGroup } from '~quent/types/ResourceGroup';
import { InlineSelector } from './InlineSelector';

const FSM_ALL = 'All';

interface ResourceGroupRowProps {
  group: ResourceGroup;
  id: string;
  availableResourceTypes?: string[];
  selectedType?: string;
  onTypeChange?: (itemId: string, type: string) => void;
  availableFsmTypes?: string[];
  selectedFsmType?: string | null;
  onFsmChange?: (itemId: string, fsmType: string | null) => void;
  verbose?: boolean;
}

export const ResourceGroupRow = ({
  group,
  id,
  availableResourceTypes,
  selectedType,
  onTypeChange,
  availableFsmTypes,
  selectedFsmType,
  onFsmChange,
}: ResourceGroupRowProps): React.ReactNode => {
  const hasMultipleChildTypes = (availableResourceTypes?.length ?? 0) > 1;
  const fsmCount = availableFsmTypes?.length ?? 0;
  const hasOneFsm = fsmCount === 1;
  const hasMultipleFsms = fsmCount > 1;
  const fsmOptions = hasMultipleFsms ? [FSM_ALL, ...(availableFsmTypes ?? [])] : [];

  return (
    <div>
      <div>
        <span className="text-sm font-bold">{group.instance_name}</span>
      </div>
      {hasMultipleChildTypes && selectedType && onTypeChange && availableResourceTypes && (
        <InlineSelector
          id={`${id}-resource-type`}
          label="Type"
          selectedType={selectedType}
          availableResourceTypes={availableResourceTypes}
          onTypeChange={onTypeChange}
          className="mt-1"
        />
      )}
      {hasOneFsm && (
        <p className="mt-1 text-xs text-muted-foreground">
          FSM: <span className="text-foreground">{availableFsmTypes![0]}</span>
        </p>
      )}
      {hasMultipleFsms && onFsmChange && fsmOptions.length > 0 && (
        <InlineSelector
          id={`${id}-fsm`}
          label="FSM"
          selectedType={selectedFsmType ?? FSM_ALL}
          availableResourceTypes={fsmOptions}
          onTypeChange={(_itemId, value) => onFsmChange(id, value === FSM_ALL ? null : value)}
          className="mt-1"
        />
      )}
    </div>
  );
};
