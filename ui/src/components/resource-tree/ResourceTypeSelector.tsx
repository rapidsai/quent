import { cn } from '@/lib/utils';

interface ResourceTypeSelectorProps {
  id: string;
  selectedType: string;
  availableResourceTypes: string[];
  onTypeChange: (itemId: string, type: string) => void;
  className?: string;
}

export const ResourceTypeSelector = ({
  id,
  selectedType,
  availableResourceTypes,
  onTypeChange,
  className,
}: ResourceTypeSelectorProps): React.ReactNode => {
  return (
    <div
      className={cn('flex items-center gap-2', className)}
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
          onTypeChange(id, e.target.value);
        }}
        className="text-xs bg-background border border-border rounded px-1 py-0.5 text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
      >
        {availableResourceTypes.map(typeOption => (
          <option key={typeOption} value={typeOption}>
            {typeOption}
          </option>
        ))}
      </select>
    </div>
  );
};
