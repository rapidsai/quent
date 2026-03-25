// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { formatDurationForWindow } from '@/services/formatters';
import { getColorForKey } from '@/services/colors';
import { cn } from '@/lib/utils';
import { nanosToMs } from '@/lib/timeline.utils';

interface TooltipSeries {
  color: string;
  name: string;
  value: number;
  isOverlay?: boolean;
}

type ValueFormatter = (value: number) => string;
const defaultFormatter: ValueFormatter = (v: number) => `${v}`;

const TooltipSeriesStat = ({
  series,
  fmt,
}: {
  series: Partial<TooltipSeries>;
  fmt: ValueFormatter;
}) => {
  return (
    <li className="flex items-center gap-1">
      {series.color && (
        <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: series.color }} />
      )}
      <span className="text-foreground">{series.name}</span>
      <span className="font-semibold ml-auto text-foreground">{fmt(series.value ?? 0)}</span>
    </li>
  );
};

interface OverlaySegment {
  name: string;
  value: number;
  color: string;
}

interface StateBar {
  state: string;
  baseValue: number;
  baseColor: string;
  overlays: OverlaySegment[];
}

interface SegmentedBarSegment {
  value: number;
  color: string;
  label: string;
  isOverlay?: boolean;
}

function SegmentedBarRow({
  label,
  segments,
  total,
  fmt,
  labelClassName,
  valueClassName,
}: {
  label: string;
  segments: SegmentedBarSegment[];
  total: number;
  fmt: ValueFormatter;
  overlayPct?: number;
  labelClassName?: string;
  valueClassName?: string;
}) {
  return (
    <>
      <span className={cn('text-foreground font-medium truncate', labelClassName)}>{label}</span>
      <div className="relative text-[11px] leading-none min-w-0" style={{ height: 12 }}>
        <div className="flex h-full rounded-xs overflow-hidden">
          {segments.map((seg, i) => {
            const pct = total > 0 ? (seg.value / total) * 100 : 100;
            const style: React.CSSProperties & Record<`--${string}`, string> = {
              width: `${pct}%`,
              textShadow: '0 0 1px hsl(var(--foreground)), 0 0 1px hsl(var(--foreground))',
              ...(seg.isOverlay ? { '--stripe-color': seg.color } : { backgroundColor: seg.color }),
            };
            return (
              <div
                key={i}
                style={style}
                className={cn(
                  'min-w-0 flex items-center justify-center font-semibold truncate text-background',
                  seg.isOverlay && 'bg-diagonal-stripe'
                )}
                title={seg.label}
              >
                {pct >= 15 ? seg.label : ''}
              </div>
            );
          })}
        </div>
      </div>
      <span className={cn('text-foreground font-semibold text-[11px] text-right', valueClassName)}>
        {fmt(total)}
      </span>
    </>
  );
}

function buildBarSegments(
  bar: StateBar,
  fmt: ValueFormatter
): {
  segments: SegmentedBarSegment[];
  overlayPct: number | undefined;
} {
  const totalOverlayValue = bar.overlays.reduce((sum, o) => sum + o.value, 0);
  const restValue = bar.baseValue - totalOverlayValue;

  const segments: SegmentedBarSegment[] = [];
  for (const o of bar.overlays) {
    if (o.value > 0) {
      segments.push({
        value: o.value,
        color: o.color,
        label: fmt(o.value),
        isOverlay: true,
      });
    }
  }
  if (restValue > 0 || segments.length === 0) {
    segments.push({
      value: Math.max(restValue, 0),
      color: bar.baseColor,
      label: fmt(Math.max(restValue, 0)),
    });
  }

  const overlayPct =
    totalOverlayValue > 0 && bar.baseValue > 0
      ? (totalOverlayValue / bar.baseValue) * 100
      : undefined;

  return { segments, overlayPct };
}

function ActiveMarksSection({ marks }: { marks: { label: string; stateName: string }[] }) {
  if (marks.length === 0) return null;
  return (
    <div className="mt-1 pt-1 border-t border-border">
      {marks.map((m, i) => (
        <div key={i} className="flex items-center gap-1">
          <span
            className="w-2 h-2 rounded-xs shrink-0 border"
            style={{
              backgroundColor: getColorForKey(m.stateName) + '20',
              borderColor: getColorForKey(m.stateName) + 'cc',
            }}
          />
          <span className="text-muted-foreground">{m.label}</span>
          <span className="text-foreground font-medium ml-auto">{m.stateName}</span>
        </div>
      ))}
    </div>
  );
}

function OverlayBarTooltip({
  timestamp,
  bars,
  startTime,
  fmt,
  windowMs,
  activeMarks,
}: {
  timestamp: number;
  bars: StateBar[];
  startTime: bigint;
  fmt: ValueFormatter;
  windowMs: number;
  activeMarks?: { label: string; stateName: string }[];
}) {
  const visibleBars = bars
    .filter(b => b.baseValue > 0 || b.overlays.some(o => o.value > 0))
    .sort((a, b) => b.baseValue - a.baseValue);

  return (
    <div
      className={cn(
        'px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50',
        { 'min-w-[240px]': visibleBars.length > 0 }
      )}
    >
      <div className="font-semibold mb-1.5 text-muted-foreground">
        {formatDurationForWindow(timestamp - nanosToMs(startTime), windowMs)}
      </div>
      <div
        className="grid items-center gap-x-1.5 gap-y-1"
        style={{ gridTemplateColumns: 'auto 1fr auto' }}
      >
        {visibleBars.map(bar => {
          const { segments, overlayPct } = buildBarSegments(bar, fmt);
          return (
            <SegmentedBarRow
              key={bar.state}
              label={bar.state}
              segments={segments}
              total={bar.baseValue}
              fmt={fmt}
              overlayPct={overlayPct}
            />
          );
        })}
        {visibleBars.length === 0 && (
          <span className="font-semibold text-[11px] text-right">Total: 0</span>
        )}
        {visibleBars.length > 1 &&
          (() => {
            const grandTotal = visibleBars.reduce((sum, b) => sum + b.baseValue, 0);
            const totalOverlay = visibleBars.reduce(
              (sum, b) => sum + b.overlays.reduce((s, o) => s + o.value, 0),
              0
            );
            const totalRest = grandTotal - totalOverlay;

            const segments: SegmentedBarSegment[] = [];
            if (totalOverlay > 0) {
              segments.push({
                value: totalOverlay,
                color: 'var(--color-gray-300)',
                label: fmt(totalOverlay),
                isOverlay: true,
              });
            }
            if (totalRest > 0 || segments.length === 0) {
              segments.push({
                value: Math.max(totalRest, 0),
                color: 'var(--color-gray-400)',
                label: fmt(Math.max(totalRest, 0)),
              });
            }

            const overlayPct =
              totalOverlay > 0 && grandTotal > 0 ? (totalOverlay / grandTotal) * 100 : undefined;

            return (
              <>
                <div className="col-span-3 border-t border-border my-0.5" />
                <SegmentedBarRow
                  label="Total"
                  segments={segments}
                  total={grandTotal}
                  fmt={fmt}
                  overlayPct={overlayPct}
                />
              </>
            );
          })()}
      </div>
      {activeMarks && <ActiveMarksSection marks={activeMarks} />}
    </div>
  );
}

export function TooltipContent({
  timestamp,
  series,
  startTime,
  fmt = defaultFormatter,
  windowMs,
  activeMarks,
}: {
  timestamp: number;
  series: TooltipSeries[];
  startTime: bigint;
  fmt?: ValueFormatter;
  windowMs: number;
  activeMarks?: { label: string; stateName: string }[];
}) {
  const hasOverlays = series.some(s => s.isOverlay);

  if (hasOverlays) {
    const baseSeries = series.filter(s => !s.isOverlay);
    const overlaySeries = series.filter(s => s.isOverlay);

    const bars: StateBar[] = baseSeries.map(base => {
      const matchingOverlays = overlaySeries.filter(o => o.name.startsWith(`${base.name} (`));
      return {
        state: base.name,
        baseValue: base.value,
        baseColor: base.color,
        overlays: matchingOverlays.map(o => ({
          name: o.name,
          value: o.value,
          color: o.color,
        })),
      };
    });

    return (
      <OverlayBarTooltip
        timestamp={timestamp}
        bars={bars}
        startTime={startTime}
        fmt={fmt}
        windowMs={windowMs}
        activeMarks={activeMarks}
      />
    );
  }

  return (
    <div className="px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50">
      <div className="font-semibold mb-1 text-muted-foreground">
        {formatDurationForWindow(timestamp - nanosToMs(startTime), windowMs)}
      </div>
      <ul>
        {series
          .sort((a, b) => a.name.localeCompare(b.name))
          .map((s, i) => (s.value > 0 ? <TooltipSeriesStat key={i} series={s} fmt={fmt} /> : null))}
      </ul>
      <section className="pt-1">
        <TooltipSeriesStat
          series={{ name: 'Total', value: series.reduce((acc, s) => acc + s.value, 0) }}
          fmt={fmt}
        />
      </section>
      {activeMarks && <ActiveMarksSection marks={activeMarks} />}
    </div>
  );
}
