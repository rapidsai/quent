import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAtom } from 'jotai';
import { selectedColorField, selectedEdgeWidthFieldAtom } from '@/atoms/dag';
import type { StatValue } from '@/services/query-plan/types';

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
  statistics: Array<Array<{ key: string; value: StatValue }>>;
  portStatFields: string[];
}

export const DAGControls = ({ statistics, portStatFields }: DAGControlsProps) => {
  const [colorField, setColorField] = useAtom(selectedColorField);
  const [edgeWidthField, setEdgeWidthField] = useAtom(selectedEdgeWidthFieldAtom);

  const colorFields = [...new Set(statistics.flatMap(node => node.map(stat => stat.key)))];

  return (
    <div className="flex p-5 gap-2 border-b">
      <DAGField
        options={colorFields}
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
