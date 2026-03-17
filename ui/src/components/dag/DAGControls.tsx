import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

const DAGField = ({ options }) => {
  console.log(options);
  return (
    <div className="flex-1">
      <Select>
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
  const fields = [...new Set(statistics.flatMap(node => node.map(stat => stat.key)))];
  console.log(fields);
  return (
    <div className="flex p-5 gap-2 border-b">
      <DAGField options={fields} />
      <DAGField />
    </div>
  );
};
