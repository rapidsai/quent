import { ChevronDown } from 'lucide-react';
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
      className={cn('flex items-center gap-1.5', className)}
      onClick={e => e.stopPropagation()}
      onMouseDown={e => e.stopPropagation()}
    >
      <label htmlFor={`type-select-${id}`} className="text-xs text-muted-foreground shrink-0">
        Type:
      </label>
      <span className="inline-flex items-center gap-1 max-w-80 min-w-0">
        <span className="relative w-fit max-w-full min-w-0 shrink inline-block">
          {/* Invisible sizer so the trigger is only as wide as the selected value text */}
          <span className="invisible whitespace-nowrap text-xs py-px" aria-hidden>
            {selectedType}
          </span>
          <select
            id={`type-select-${id}`}
            value={selectedType}
            onChange={e => {
              e.stopPropagation();
              onTypeChange(id, e.target.value);
            }}
            className={cn(
              'absolute left-0 top-0 w-full h-full text-xs text-foreground bg-transparent border-none border-b border-dashed border-muted-foreground/60',
              'cursor-pointer focus:outline-none focus:border-muted-foreground',
              'appearance-none py-px pr-0'
            )}
          >
            {availableResourceTypes.map(typeOption => (
              <option key={typeOption} value={typeOption}>
                {typeOption}
              </option>
            ))}
          </select>
        </span>
        <ChevronDown
          className="h-3 w-3 shrink-0 text-muted-foreground pointer-events-none"
          aria-hidden
        />
      </span>
    </div>
  );
};
