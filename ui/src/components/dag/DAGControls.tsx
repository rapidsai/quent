import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAtom } from 'jotai';
import { selectedColorField, selectedEdgeWidthFieldAtom } from '@/atoms/dag';
interface DAGFieldProps {
  options: string[];
  value: string;
  setValue: (value: string) => void;
  placeholder: string;
}

const DAGField = ({ options, value, setValue, placeholder }: DAGFieldProps) => {
  return (
    <div className="flex-1">
      <Select value={value} onValueChange={setValue}>
        <SelectTrigger>
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent>
          {options.length === 0 ? (
            <SelectItem value="_empty" disabled>
              No data available
            </SelectItem>
          ) : (
            options.map(opt => (
              <SelectItem key={opt} value={opt}>
                {opt}
              </SelectItem>
            ))
          )}
        </SelectContent>
      </Select>
    </div>
  );
};

interface DAGControlsProps {
  operatorStatFields: string[];
  portStatFields: string[];
}

export const DAGControls = ({ operatorStatFields, portStatFields }: DAGControlsProps) => {
  const [colorField, setColorField] = useAtom(selectedColorField);
  const [edgeWidthField, setEdgeWidthField] = useAtom(selectedEdgeWidthFieldAtom);

  return (
    <div className="flex p-5 gap-2 border-b">
      <DAGField
        options={operatorStatFields}
        value={colorField ?? ''}
        setValue={setColorField}
        placeholder="Color nodes by field"
      />
      <DAGField
        options={portStatFields}
        value={edgeWidthField ?? ''}
        setValue={setEdgeWidthField}
        placeholder="Edge width by field"
      />
    </div>
  );
};
