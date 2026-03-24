import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  SelectGroup,
} from '@/components/ui/select';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useAtom } from 'jotai';
import { selectedColorField, selectedEdgeWidthFieldAtom } from '@/atoms/dag';
import { Palette, Spline, X } from 'lucide-react';

interface DAGFieldProps {
  label: string;
  icon: React.ElementType;
  options: string[];
  value: string;
  setValue: (value: string | null) => void;
  placeholder: string;
}

const DAGField = ({ label, icon: Icon, options, value, setValue, placeholder }: DAGFieldProps) => (
  <div className="flex flex-col gap-1 min-w-0 flex-1">
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-1 text-xs text-muted-foreground">
        <Icon className="h-3 w-3 shrink-0" />
        <span>{label}</span>
      </div>
      {value && (
        <button
          onClick={() => setValue(null)}
          className="text-muted-foreground hover:text-foreground transition-colors"
          aria-label={`Clear ${label}`}
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </div>
    <Select value={value} onValueChange={setValue}>
      <SelectTrigger className="h-7 text-xs">
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        <ScrollArea viewportClassName="max-h-[10rem]">
          <SelectGroup>
            {options.length === 0 ? (
              <SelectItem value="_empty" disabled>
                No data available
              </SelectItem>
            ) : (
              options.map(opt => (
                <SelectItem key={opt} value={opt} className="text-xs">
                  {opt}
                </SelectItem>
              ))
            )}
          </SelectGroup>
        </ScrollArea>
      </SelectContent>
    </Select>
  </div>
);

interface DAGControlsProps {
  operatorStatFields: string[];
  portStatFields: string[];
}

export const DAGControls = ({ operatorStatFields, portStatFields }: DAGControlsProps) => {
  const [colorField, setColorField] = useAtom(selectedColorField);
  const [edgeWidthField, setEdgeWidthField] = useAtom(selectedEdgeWidthFieldAtom);

  return (
    <div className="flex items-end px-4 py-3 gap-3 border-b bg-card">
      <DAGField
        label="Node color"
        icon={Palette}
        options={operatorStatFields}
        value={colorField ?? ''}
        setValue={setColorField}
        placeholder="None"
      />
      <DAGField
        label="Edge width"
        icon={Spline}
        options={portStatFields}
        value={edgeWidthField ?? ''}
        setValue={setEdgeWidthField}
        placeholder="None"
      />
    </div>
  );
};
