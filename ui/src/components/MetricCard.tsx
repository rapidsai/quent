import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { ArrowUpIcon, ArrowDownIcon } from 'lucide-react';
import { cn } from '@/lib/utils';

interface MetricCardProps {
  title: string;
  value: string | number;
  trend?: number;
  icon?: string;
}

export function MetricCard({ title, value, trend, icon }: MetricCardProps) {
  const trendColor =
    trend && trend > 0 ? 'text-green-600' : trend && trend < 0 ? 'text-red-600' : 'text-gray-500';

  return (
    <Card className="transition-all hover:shadow-lg">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {title}
          </span>
          {icon && <span className="text-2xl">{icon}</span>}
        </div>
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-bold text-foreground mb-2">{value}</div>
        {trend !== undefined && (
          <div className={cn('flex items-center gap-1 text-sm font-semibold', trendColor)}>
            {trend > 0 ? (
              <ArrowUpIcon className="h-4 w-4" />
            ) : trend < 0 ? (
              <ArrowDownIcon className="h-4 w-4" />
            ) : null}
            <span>{Math.abs(trend)}%</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
