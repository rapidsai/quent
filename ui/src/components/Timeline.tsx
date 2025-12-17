import { useMemo } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
interface TooltipSeries {
  color: string;
  name: string;
  value: number;
}

function TooltipContent({ date, series }: { date: Date; series: TooltipSeries[] }) {
  const formattedDate = date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });

  return (
    <div className="p-2">
      <div className="font-semibold mb-2">{formattedDate}</div>
      <div className="flex-col gap-4">
        {series.map((s, i) => (
          <div key={i} className="flex items-center gap-4">
            <span
              className={`w-2 h-2 rounded-full flex-shrink-0`}
              // Not possible w tailwind
              style={{ backgroundColor: s.color }}
            />
            <span>{s.name}:</span>
            <strong>{Math.round(s.value)}</strong>
          </div>
        ))}
      </div>
    </div>
  );
}

export function Timeline({
  series,
  timestamps,
}: {
  series: Record<string, number[]>;
  timestamps: number[];
}) {
  const seriesOptions = useMemo(() => {
    return Object.entries(series).map(([name, data], index) => ({
      name,
      type: 'line',
      step: 'middle',
      symbol: 'none',
      xAxisIndex: index,
      yAxisIndex: index,
      data,
    }));
  }, [series]);

  const yAxisOptions = useMemo(() => {
    return Object.entries(series).map(([name], idx) => ({
      name,
      type: 'value',
      boundaryGap: false,
      splitNumber: 1,
      ...(idx === 0 ? {} : { gridIndex: idx }),
    }));
  }, [series]);

  const xAxisOptions = useMemo(() => {
    return Object.keys(series).map((_, idx) => ({
      data: timestamps,
      boudnaryGap: false,
      type: 'category',
      axisLabel: {
        show: false,
      },
      ...(idx === 0 ? {} : { gridIndex: idx }),
    }));
    // See if we can reduce this to just one xAxis option
  }, [timestamps, series]);

  const gridOptions = useMemo(() => {
    const timelineHeight = 100;
    const timelinePadding = 50;
    const backgroundColor = 'rgba(128, 128, 128, 0.1)';
    return Object.keys(series).map((_, idx) => ({
      left: 40,
      right: 10,
      top: `${(idx + 1) * timelinePadding + timelineHeight * idx}px`,
      height: `${timelineHeight}px`,
      backgroundColor,
      show: true,
    }));
  }, [series]);

  const eChartOptions: EChartsOption = useMemo(() => {
    return {
      tooltip: {
        trigger: 'axis',
        position: function (pt) {
          return [pt[0], '10%'];
        },
        formatter: function (params: unknown) {
          if (!Array.isArray(params) || params.length === 0) return '';
          const timestamp = Number(params[0].axisValue);
          const date = new Date(timestamp);
          const series = params.map((p: { color: string; seriesName: string; value: number }) => ({
            color: p.color,
            name: p.seriesName,
            value: p.value,
          }));
          return renderToStaticMarkup(<TooltipContent date={date} series={series} />);
        },
      },
      title: {
        left: 'center',
      },
      axisPointer: {
        link: [
          {
            xAxisIndex: 'all',
          },
        ],
      },
      grid: gridOptions,
      dataZoom: [
        {
          show: true,
          realtime: true,
          start: 0,
          end: 10,
          xAxisIndex: [0, 1, 2],
        },
        {
          type: 'inside',
          realtime: true,
          start: 0,
          end: 10,
          xAxisIndex: [0, 1, 2],
        },
      ],
      xAxis: xAxisOptions,
      yAxis: yAxisOptions,
      series: seriesOptions,
    } as EChartsOption;
  }, [seriesOptions, yAxisOptions, xAxisOptions, gridOptions]);

  return <ReactECharts option={eChartOptions} style={{ height: '600px' }} notMerge lazyUpdate />;
}
