import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export interface LineChartProps {
  data: Array<{ date: string; value: number }>;
  title?: string;
  height?: string;
  color?: string;
}

export function LineChart({
  data,
  title = 'Line Chart',
  height = '400px',
  color = '#5470c6',
}: LineChartProps) {
  const option: EChartsOption = {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'cross',
      },
    },
    xAxis: {
      type: 'category',
      data: data.map(item => item.date),
      boundaryGap: false,
      axisLabel: {
        rotate: 45,
        fontSize: 10,
      },
    },
    yAxis: {
      type: 'value',
      name: 'Value',
    },
    series: [
      {
        data: data.map(item => item.value),
        type: 'line',
        symbol: 'circle',
        symbolSize: 6,
        itemStyle: {
          color: color,
        },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              {
                offset: 0,
                color: color + '80', // Add transparency
              },
              {
                offset: 1,
                color: color + '10',
              },
            ],
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
