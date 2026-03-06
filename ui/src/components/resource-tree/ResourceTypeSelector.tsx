import { cn } from '@/lib/utils';

interface ResourceTypeSelectorProps {
  id: string;
  selectedType: string;
  availableResourceTypes: string[];
  onTypeChange: (itemId: string, type: string) => void;
  compact?: boolean;
  className?: string;
}

export const ResourceTypeSelector = ({
  id,
  selectedType,
  availableResourceTypes,
  onTypeChange,
  compact,
  className,
}: ResourceTypeSelectorProps): React.ReactNode => {
  return (
    <div
      className={cn('flex items-center gap-1.5', className)}
      onClick={e => e.stopPropagation()}
      onMouseDown={e => e.stopPropagation()}
    >
      {!compact && (
        <label htmlFor={`type-select-${id}`} className="text-xs text-muted-foreground">
          Type:
        </label>
      )}
      <select
        id={`type-select-${id}`}
        value={selectedType}
        onChange={e => {
          e.stopPropagation();
          onTypeChange(id, e.target.value);
        }}
        className={cn(
          'text-xs bg-background border border-border rounded text-foreground focus:outline-none focus:ring-1 focus:ring-ring',
          // compact ? 'px-0.5 py-0 text-[10px] leading-tight' : 'px-1 py-0.5'
          'px-1 py-0.5'
        )}
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
