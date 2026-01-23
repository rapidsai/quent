import { formatBytes, formatDuration } from '@/services/formatters';

interface TooltipSeries {
  color: string;
  name: string;
  value: number;
}

const TooltipSeriesStat = ({ series }: { series: Partial<TooltipSeries> }) => {
  return (
    <li className="flex items-center gap-1">
      {series.color && (
        <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: series.color }} />
      )}
      <span className="text-foreground">{series.name}</span>
      <span className="font-semibold ml-auto text-foreground">
        {formatBytes(series.value ?? 0, 2)}
      </span>
    </li>
  );
};

export function TooltipContent({
  timestamp,
  series,
  startTime,
}: {
  timestamp: number;
  series: TooltipSeries[];
  startTime: bigint;
}) {
  return (
    <div className="px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50">
      <div className="font-semibold mb-1 text-muted-foreground">
        {formatDuration(Number(BigInt(timestamp) - startTime / 1_000_000n))}
      </div>
      <ul>
        {series
          .sort((a, b) => a.name.localeCompare(b.name))
          .map((s, i) => (s.value > 0 ? <TooltipSeriesStat key={i} series={s} /> : null))}
      </ul>
      <section className="pt-1">
        <TooltipSeriesStat
          series={{ name: 'Total', value: series.reduce((acc, s) => acc + s.value, 0) }}
        />
      </section>
    </div>
  );
}
