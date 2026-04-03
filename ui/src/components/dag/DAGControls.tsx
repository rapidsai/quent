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
import {
  selectedColorField,
  selectedEdgeWidthFieldAtom,
  selectedEdgeColorFieldAtom,
  selectedNodeLabelFieldAtom,
  NODE_LABEL_FIELD,
  type NodeLabelField,
} from '@/atoms/dag';
import { Palette, Spline, X, Brush, ChevronDown, Type } from 'lucide-react';
import { DAGSettingsPopover } from './DAGSettingsPopover';
import { useState } from 'react';

interface DAGFieldProps {
  label: string;
  icon: React.ElementType;
  options: string[];
  value: string;
  setValue: (value: string | null) => void;
  placeholder: string;
  clearable?: boolean;
  optionLabels?: Record<string, string>;
}

const DAGField = ({
  label,
  icon: Icon,
  options,
  value,
  setValue,
  placeholder,
  clearable = true,
  optionLabels,
}: DAGFieldProps) => (
  <div className="flex items-center gap-1.5 min-w-0">
    <Icon className="h-3 w-3 shrink-0 text-muted-foreground" />
    <span className="text-xs text-muted-foreground shrink-0 whitespace-nowrap">{label}</span>
    <Select value={value} onValueChange={setValue}>
      <SelectTrigger className="h-6 text-xs flex-1 min-w-0">
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
                  {optionLabels?.[opt] ?? opt}
                </SelectItem>
              ))
            )}
          </SelectGroup>
        </ScrollArea>
      </SelectContent>
    </Select>
    {clearable && value && (
      <button
        onClick={() => setValue(null)}
        className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
        aria-label={`Clear ${label}`}
      >
        <X className="h-3 w-3" />
      </button>
    )}
  </div>
);

interface DAGControlsProps {
  operatorStatFields: string[];
  portStatFields: string[];
}

const NODE_LABEL_OPTIONS = [NODE_LABEL_FIELD.NAME, NODE_LABEL_FIELD.ID, NODE_LABEL_FIELD.TYPE];
const NODE_LABEL_DISPLAY: Record<string, string> = { name: 'Name', id: 'ID', type: 'Type' };

export const DAGControls = ({ operatorStatFields, portStatFields }: DAGControlsProps) => {
  const [colorField, setColorField] = useAtom(selectedColorField);
  const [edgeWidthField, setEdgeWidthField] = useAtom(selectedEdgeWidthFieldAtom);
  const [edgeColorField, setEdgeColorField] = useAtom(selectedEdgeColorFieldAtom);
  const [nodeLabelField, setNodeLabelField] = useAtom(selectedNodeLabelFieldAtom);
  const [open, setOpen] = useState(true);

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="border-b bg-card">
      <div className="flex items-center justify-between px-4 py-2">
        <CollapsibleTrigger className="flex items-center gap-2 group">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            DAG Controls
          </span>
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground transition-transform duration-200 cursor-pointer group-data-[state=open]:rotate-180" />
        </CollapsibleTrigger>
        <DAGSettingsPopover />
      </div>
      <CollapsibleContent className="px-4 pb-2 grid grid-cols-2 gap-x-3 gap-y-1.5">
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
        <DAGField
          label="Edge color"
          icon={Brush}
          options={portStatFields}
          value={edgeColorField ?? ''}
          setValue={setEdgeColorField}
          placeholder="None"
        />
        <DAGField
          label="Node label"
          icon={Type}
          options={NODE_LABEL_OPTIONS}
          optionLabels={NODE_LABEL_DISPLAY}
          value={nodeLabelField}
          setValue={v => v && setNodeLabelField(v as NodeLabelField)}
          placeholder="Name"
          clearable={false}
        />
      </CollapsibleContent>
    </Collapsible>
  );
};
