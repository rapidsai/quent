import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export interface BarChartProps {
  data: Array<{ category: string; value: number }>;
  title?: string;
  height?: string;
  color?: string;
}

export function BarChart({
  data,
  title = 'Bar Chart',
  height = '400px',
  color = '#91cc75',
}: BarChartProps) {
  const option: EChartsOption = {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'shadow',
      },
    },
    xAxis: {
      type: 'category',
      data: data.map(item => item.category),
      axisLabel: {
        interval: 0,
        rotate: 30,
        fontSize: 10,
      },
    },
    yAxis: {
      type: 'value',
      name: 'Value',
    },
    series: [
      {
        data: data.map(item => ({
          value: item.value,
          itemStyle: {
            color: color,
          },
        })),
        type: 'bar',
        showBackground: true,
        backgroundStyle: {
          color: 'rgba(180, 180, 180, 0.2)',
        },
        emphasis: {
          itemStyle: {
            color: '#fac858',
          },
        },
      },
    ],
    grid: {
      left: '3%',
      right: '4%',
      bottom: '15%',
      containLabel: true,
    },
  };

  return (
    <Card className="transition-all hover:shadow-lg">
      <CardHeader>
        <CardTitle className="text-lg">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <ReactECharts option={option} style={{ height }} notMerge={true} lazyUpdate={true} />
      </CardContent>
    </Card>
  );
}
