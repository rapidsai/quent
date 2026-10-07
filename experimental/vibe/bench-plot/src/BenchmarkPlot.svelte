<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { onMount } from 'svelte';
  import * as echarts from 'echarts/core';
  import { BarChart, BoxplotChart, ScatterChart } from 'echarts/charts';
  import { GraphicComponent, GridComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import type { EChartsType } from 'echarts/core';
  import type { BenchCase, BoxSummary } from './report';
  import type { PlotMode } from './plotPreference.svelte';
  import { batchAverages, boxSummary, caseAxisLabel, caseLabel, caseLanguage, discardedCallCount, frameworkColor, hasDiscardedCalls, isNoopCase } from './report';
  import { jitterOffset } from './jitter';

  echarts.use([BarChart, BoxplotChart, ScatterChart, GraphicComponent, GridComponent, TooltipComponent, CanvasRenderer]);

  let { cases, title, mode, colors }: {
    cases: BenchCase[];
    title: string;
    mode: PlotMode;
    colors: ReadonlyMap<string, string>;
  } = $props();

  let element: HTMLDivElement;
  let chart = $state<EChartsType | undefined>(undefined);

  let showLanguages = $derived(new Set(cases.map(caseLanguage)).size > 1);
  let labels = $derived(cases.map((item) => caseAxisLabel(item, showLanguages)));
  let current = $derived(mode === 'advanced' ? cases.map(boxSummary) : []);
  let samples = $derived(mode === 'advanced' ? cases.map(batchAverages) : []);
  const tickFormatter = new Intl.NumberFormat('en-US', { maximumSignificantDigits: 3 });
  $effect(() => {
    if (chart) render(current, samples, labels, mode, colors);
  });

  function coordinates(summary: BoxSummary): number[] {
    return [summary.low, summary.q1, summary.median, summary.q3, summary.high];
  }

  function caseColor(item: BenchCase, colors: ReadonlyMap<string, string>, lightness?: number): string {
    return frameworkColor(item.implementation, colors, lightness, hasDiscardedCalls(item) ? 0.4 : 1);
  }

  function render(current: BoxSummary[], samples: number[][], labels: string[], mode: PlotMode,
    colors: ReadonlyMap<string, string>) {
    const grid = { left: 60, right: 14, top: 30, bottom: showLanguages ? 100 : 84 };
    const bandWidth = Math.max(1, (element.clientWidth - grid.left - grid.right) / Math.max(1, labels.length));
    const noopBackgrounds = cases.flatMap((item, index) => isNoopCase(item) ? [{
      type: 'rect', silent: true, z: 0,
      shape: { x: grid.left + index * bandWidth, y: grid.top, width: bandWidth,
        height: Math.max(0, element.clientHeight - grid.top - grid.bottom) },
      style: { fill: 'rgba(107, 114, 128, 0.12)' },
    }] : []);
    const xAxis = { type: 'category', data: labels,
      axisLabel: { interval: 0, width: Math.min(70, Math.max(24, bandWidth - 4)), overflow: 'break',
        fontSize: 10, lineHeight: 12,
        formatter: (label: string) => {
          const formatted = label.replaceAll(' / ', '\n');
          return formatted.startsWith('⚠ ') ? `{warning|⚠} ${formatted.slice(2)}` : formatted;
        },
        rich: { warning: { color: '#B45309', fontSize: 15, fontWeight: 'bold', lineHeight: 16 } } },
      axisTick: { alignWithLabel: true } };
    if (mode === 'simple') {
      chart?.setOption({
        animation: false,
        grid,
        graphic: noopBackgrounds,
        xAxis,
        yAxis: { type: 'value', min: 0, max: ({ max }: { max: number }) => max > 0 ? max * 1.15 : 1,
          name: 'Average ns / call', nameLocation: 'middle', nameGap: 44,
          axisLabel: { formatter: (value: number) => tickFormatter.format(value) } },
        tooltip: { trigger: 'item', formatter: (params: { dataIndex: number }) => {
          const source = cases[params.dataIndex];
          if (!source) return '';
          const discarded = discardedCallCount(source);
          return `${caseLabel(source, showLanguages)}<br/>Average: ${source.average_ns_per_iteration.toFixed(2)} ns${discarded ? `<br/>Discarded calls: ${discarded}` : ''}`;
        } },
        series: [{ name: 'Average', type: 'bar', barMaxWidth: 72,
          label: { show: true, position: 'top', fontSize: 10, formatter: (params: { value: number }) =>
            tickFormatter.format(params.value) },
          data: cases.map((item) => ({ value: item.average_ns_per_iteration,
            itemStyle: { color: caseColor(item, colors),
              opacity: isNoopCase(item) ? 0.55 : 1 } })) }],
      }, true);
      return;
    }
    const availableWidth = Math.max(4, bandWidth * 0.8 - 2);
    const boxWidth = Math.min(72, Math.max(2, availableWidth * 0.2));
    // A second, empty boxplot series moves the real box to the left of the category center.
    const boxCenter = -availableWidth * 0.2875;
    const sampleLeft = boxCenter + boxWidth / 2 + Math.min(3, availableWidth * 0.05);
    const sampleRight = availableWidth / 2;
    const sampleCenter = (sampleLeft + sampleRight) / 2;
    const sampleWidth = Math.max(2, sampleRight - sampleLeft);
    const logarithmic = samples.every((values) => values.every((value) => value > 0));
    const dotSize = bandWidth < 30 ? 2 : 3;
    const sampleData = samples.flatMap((values, index) => {
      const itemStyle = { color: caseColor(cases[index], colors, 80),
        opacity: isNoopCase(cases[index]) ? 0.5 : 1 };
      return values.map((sample, sampleIndex) => ({
        value: [index, sample],
        symbolOffset: [sampleCenter + jitterOffset(`${labels[index]}:${sample}:${sampleIndex}`, sampleWidth, dotSize), 0],
        itemStyle,
      }));
    });
    const averageLabels = current.map((summary, index) => ({
      value: [index, summary.high],
      symbolOffset: [boxCenter, 0],
      label: {
        show: true,
        position: 'top',
        distance: 5,
        formatter: cases[index].average_ns_per_iteration.toFixed(1),
        color: caseColor(cases[index], colors),
        opacity: isNoopCase(cases[index]) ? 0.6 : 1,
        fontSize: 11,
        fontWeight: 600,
      },
      itemStyle: { color: 'transparent' },
    }));
    const series: Array<Record<string, unknown>> = [
      {
        name: 'Results', type: 'boxplot', z: 4, boxWidth: [boxWidth, boxWidth],
        data: current.map((summary, index) => ({
          value: coordinates(summary),
          itemStyle: {
            color: caseColor(cases[index], colors, 69),
            borderColor: caseColor(cases[index], colors),
            borderWidth: 2,
            opacity: isNoopCase(cases[index]) ? 0.6 : 1,
          },
        })),
      },
      {
        name: 'Sample space', type: 'boxplot', silent: true, tooltip: { show: false },
        data: cases.map(() => '-'),
      },
      {
        name: 'Batch samples', type: 'scatter', z: 2, symbolSize: dotSize,
        progressive: 2000, progressiveThreshold: 5000,
        data: sampleData,
        tooltip: { show: false },
      },
      {
        name: 'Average', type: 'scatter', z: 6, symbolSize: 1, silent: true,
        data: averageLabels,
        tooltip: { show: false },
      },
    ];
    chart?.setOption({
      animation: false,
      grid,
      graphic: noopBackgrounds,
      xAxis,
      yAxis: { type: logarithmic ? 'log' : 'value', logBase: 10,
        min: logarithmic ? ({ min }: { min: number }) => min / 1.2 : 0,
        max: ({ max }: { max: number }) => max > 0 ? max * 1.2 : 1,
        name: logarithmic ? 'ns / call (log scale)' : 'ns / call', nameLocation: 'middle', nameGap: 44,
        axisLabel: { formatter: (value: number) => tickFormatter.format(value) } },
      tooltip: { trigger: 'item', formatter: (params: { dataIndex: number }) => {
        const summary = current[params.dataIndex];
        const source = cases[params.dataIndex];
        if (!summary || !source) return '';
        const discarded = discardedCallCount(source);
        return `${caseLabel(source, showLanguages)}<br/>Average: ${source.average_ns_per_iteration.toFixed(2)} ns<br/>Median: ${summary.median.toFixed(2)} ns${discarded ? `<br/>Discarded calls: ${discarded}` : ''}`;
      } },
      series,
    }, true);
  }

  onMount(() => {
    chart = echarts.init(element, undefined, { renderer: 'canvas' });
    const observer = new ResizeObserver(() => { chart?.resize(); render(current, samples, labels, mode, colors); });
    observer.observe(element);
    return () => { observer.disconnect(); chart?.dispose(); };
  });
</script>

<div class="h-[280px] w-full min-w-0" bind:this={element} role="img" aria-label={`${title}: ${mode === 'advanced' ? 'box plots' : 'bars of average latency'} by framework and exporter`}>
</div>
