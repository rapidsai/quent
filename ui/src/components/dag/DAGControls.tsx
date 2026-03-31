import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  SelectGroup,
} from '@/components/ui/select';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useAtom } from 'jotai';
import { selectedColorField, selectedEdgeWidthFieldAtom, selectedEdgeColorFieldAtom, selectedNodeDisplayFieldAtom } from '@/atoms/dag';
import { Palette, Spline, X, Brush, Tag, ChevronDown } from 'lucide-react';
import { DAGSettingsPopover } from './DAGSettingsPopover';
import { useState } from 'react';

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
  const [edgeColorField, setEdgeColorField] = useAtom(selectedEdgeColorFieldAtom);
  const [nodeDisplayField, setNodeDisplayField] = useAtom(selectedNodeDisplayFieldAtom);
  const [open, setOpen] = useState(true);

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="border-b bg-card">
      <div className="flex items-center justify-between px-4 py-3">
        <CollapsibleTrigger className="flex items-center gap-2 group">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">DAG Controls</span>
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground transition-transform duration-200 group-data-[state=open]:rotate-180" />
        </CollapsibleTrigger>
        <DAGSettingsPopover />
      </div>
      <CollapsibleContent className="px-4 pb-3 flex flex-col gap-2">
        <div className="flex items-end gap-3">
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
        <div className="flex items-end gap-3">
          <DAGField
            label="Edge color"
            icon={Brush}
            options={portStatFields}
            value={edgeColorField ?? ''}
            setValue={setEdgeColorField}
            placeholder="None"
          />
          <DAGField
            label="Display field"
            icon={Tag}
            options={operatorStatFields}
            value={nodeDisplayField ?? ''}
            setValue={setNodeDisplayField}
            placeholder="None"
          />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
};
