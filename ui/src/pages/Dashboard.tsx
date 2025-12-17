import { useQuery } from '@tanstack/react-query';
import { LineChart } from '@/components/LineChart';
import { BarChart } from '@/components/BarChart';
import { fetchLineChartData, fetchBarChartData } from '@/services/api';

export default function Dashboard() {
  // Fetch data using TanStack Query
  const { data: lineData, isLoading: lineLoading } = useQuery({
    queryKey: ['lineChartData'],
    queryFn: fetchLineChartData,
  });

  const { data: barData, isLoading: barLoading } = useQuery({
    queryKey: ['barChartData'],
    queryFn: fetchBarChartData,
  });

  return (
    <div className="w-full space-y-8">
      <div className="grid gap-6 grid-cols-[1fr_1fr]">
        <div className="col-span-1 flex flex-col gap-6 md:col-span-1">
          <div className="w-full">
            {barLoading ? (
              <div className="flex justify-center items-center min-h-[400px] text-muted-foreground">
                Loading bar chart...
              </div>
            ) : barData ? (
              <BarChart data={barData} title="Category Comparison" color="#91cc75" />
            ) : (
              <div className="flex justify-center items-center min-h-[400px] text-destructive">
                Failed to load bar chart data
              </div>
            )}
          </div>
        </div>
        <div className="col-span-1 flex flex-col gap-6 md:col-span-1">
          <div className="w-full">
            {lineLoading ? (
              <div className="flex justify-center items-center min-h-[400px] text-muted-foreground">
                Loading line chart...
              </div>
            ) : lineData ? (
              <LineChart data={lineData} title="Trend Over Time" color="#5470c6" />
            ) : (
              <div className="flex justify-center items-center min-h-[400px] text-destructive">
                Failed to load line chart data
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Query Plan DAG */}
      <div className="col-span-full"></div>
    </div>
  );
}
