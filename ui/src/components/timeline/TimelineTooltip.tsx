import { formatBytes } from '@/services/formatters';

interface TooltipSeries {
  color: string;
  name: string;
  value: number;
}

export function TooltipContent({ date, series }: { date: Date; series: TooltipSeries[] }) {
  const formattedDate = date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });

  return (
    <div className="px-2 py-1.5 bg-popover rounded text-[11px] text-foregroundleading-tight shadow-md">
      <div className="font-semibold mb-1 text-muted-foreground">{formattedDate}</div>
      {series.map((s, i) => (
        <div key={i} className="flex items-center gap-1.5 mb-0.5 last:mb-0">
          <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: s.color }} />
          <span className="text-foreground">{s.name}</span>
          <span className="font-semibold ml-auto text-foreground">{formatBytes(s.value, 2)}</span>
        </div>
      ))}
    </div>
  );
}
