import { useState } from 'react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAtomValue, useStore } from 'jotai';
import { selectedColorField } from '@/atoms/dag';

const DAGField = ({ options, value, setValue }) => {
  console.log(options);
  return (
    <div className="flex-1">
      <Select value={value} onValueChange={setValue}>
        <SelectTrigger>
          <SelectValue placeholder="Select color field" />
        </SelectTrigger>
        <SelectContent>
          {!options || options.length === 0 ? (
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

export const DAGControls = ({ statistics }) => {
  const [selectedField, setSelectedField] = useState('');
  const store = useStore();
  const fields = [...new Set(statistics.flatMap(node => node.map(stat => stat.key)))];
  console.log(fields);

  const setValue = field => {
    setSelectedField(field);
    store.set(selectedColorField, field);
  };

  return (
    <div className="flex p-5 gap-2 border-b">
      <DAGField options={fields} value={selectedField} setValue={setValue} />
      <DAGField />
    </div>
  );
};
