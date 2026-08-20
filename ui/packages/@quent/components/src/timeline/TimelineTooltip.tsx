// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  formatDurationForWindow,
  formatDuration,
  formatAttributeValue,
  cn,
  type DynamicAttribute,
} from '@quent/utils';
import { ColorSwatch } from '../ui/color-swatch';
import { DataText } from '../ui/data-text';

/** A timeline mark under the hover cursor, as shown in the tooltip. */
export interface ActiveMark {
  label: string;
  stateName: string;
  color: string;
  /** Attributes recorded by instrumentation on the hovered state. */
  attributes?: DynamicAttribute[];
  /** Attributes computed by the analyzer. */
  derivedAttributes?: DynamicAttribute[];
  /** Duration of the hovered state span in milliseconds. */
  durationMs?: number;
}

interface TooltipSeries {
  color: string;
  name: string;
  value: number;
  isOverlay?: boolean;
  isDimmed?: boolean;
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
      {series.color && <ColorSwatch color={series.color} />}
      <DataText className="text-foreground">{series.name}</DataText>
      <DataText className="font-semibold ml-auto text-foreground">
        {fmt(series.value ?? 0)}
      </DataText>
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
  isDimmed?: boolean;
}

interface SegmentedBarSegment {
  value: number;
  color: string;
  label: string;
  /** When true, this segment is the non-operator "rest" and is rendered at low opacity. */
  isDimmed?: boolean;
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
      <DataText
        className={cn('text-foreground font-medium truncate tracking-tight', labelClassName)}
      >
        {label}
      </DataText>
      <div className="relative text-[11px] leading-none min-w-0" style={{ height: 12 }}>
        <div className="flex h-full rounded-xs overflow-hidden">
          {segments.map((seg, i) => {
            const pct = total > 0 ? (seg.value / total) * 100 : 100;
            // For dimmed segments, bake the alpha into the background so
            // text stays fully opaque and readable.
            const bgColor = seg.isDimmed
              ? `color-mix(in srgb, ${seg.color} 30%, transparent)`
              : seg.color;
            const textColor = seg.isDimmed ? 'text-foreground' : 'text-background';
            const style: React.CSSProperties = {
              width: `${pct}%`,
              backgroundColor: bgColor,
            };
            return (
              <div
                key={i}
                style={style}
                className={cn(
                  'min-w-0 flex items-center justify-center font-semibold truncate',
                  textColor
                )}
                title={seg.label}
              >
                <DataText className="tracking-tighter">{pct >= 30 ? seg.label : ''}</DataText>
              </div>
            );
          })}
        </div>
      </div>
      <DataText
        className={cn(
          'text-foreground font-semibold text-[11px] text-right tracking-tighter',
          valueClassName
        )}
      >
        {fmt(total)}
      </DataText>
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
      });
    }
  }
  if (restValue > 0 || segments.length === 0) {
    segments.push({
      value: Math.max(restValue, 0),
      color: bar.baseColor,
      label: fmt(Math.max(restValue, 0)),
      isDimmed: bar.isDimmed,
    });
  }

  const overlayPct =
    totalOverlayValue > 0 && bar.baseValue > 0
      ? (totalOverlayValue / bar.baseValue) * 100
      : undefined;

  return { segments, overlayPct };
}

/** Values longer than this wrap onto their own line. */
const INLINE_VALUE_MAX_CHARS = 32;

function MarkDetailRow({ name, value }: { name: string; value: string }) {
  if (value.length > INLINE_VALUE_MAX_CHARS) {
    return (
      <div className="pl-3">
        <DataText className="text-muted-foreground">{name}</DataText>
        <DataText as="div" className="text-foreground break-words pl-2">
          {value}
        </DataText>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-1 pl-3">
      <DataText className="text-muted-foreground">{name}</DataText>
      <DataText className="text-foreground ml-auto">{value}</DataText>
    </div>
  );
}

const ACTIVE_MARK_LIMIT = 6;

function ActiveMarksSection({ marks }: { marks: ActiveMark[] }) {
  if (marks.length === 0) return null;
  const visibleMarks = marks.slice(0, ACTIVE_MARK_LIMIT);
  const hiddenCount = marks.length - visibleMarks.length;

  return (
    <div className="mt-1 pt-1 border-t border-border">
      {visibleMarks.map((m, i) => (
        <div key={i}>
          <div className="flex items-center gap-1">
            <ColorSwatch color={m.color} />
            <DataText className="text-muted-foreground">{m.label}</DataText>
            <DataText className="text-foreground font-medium ml-auto">{m.stateName}</DataText>
          </div>
          {m.durationMs !== undefined && (
            <MarkDetailRow name="duration" value={formatDuration(m.durationMs)} />
          )}
          {m.attributes?.map(attr => (
            <MarkDetailRow
              key={attr.key}
              name={attr.key}
              value={formatAttributeValue(attr.key, attr.value)}
            />
          ))}
          {m.derivedAttributes && m.derivedAttributes.length > 0 && (
            <>
              <DataText as="div" className="pl-3 pt-0.5 text-muted-foreground italic opacity-70">
                derived
              </DataText>
              {m.derivedAttributes.map(attr => (
                <MarkDetailRow
                  key={attr.key}
                  name={attr.key}
                  value={formatAttributeValue(attr.key, attr.value)}
                />
              ))}
            </>
          )}
        </div>
      ))}
      {hiddenCount > 0 && (
        <DataText as="div" className="pt-1 text-muted-foreground">
          {hiddenCount} more {hiddenCount === 1 ? 'entity' : 'entities'} not shown
        </DataText>
      )}
    </div>
  );
}

function OverlayBarTooltip({
  timestamp,
  bars,
  fmt,
  windowMs,
  activeMarks,
}: {
  timestamp: number;
  bars: StateBar[];
  fmt: ValueFormatter;
  windowMs: number;
  activeMarks?: ActiveMark[];
}) {
  const visibleBars = bars
    .filter(b => b.baseValue > 0 || b.overlays.some(o => o.value > 0))
    .sort((a, b) => b.baseValue - a.baseValue);

  return (
    <div
      className={cn(
        'px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50',
        { 'min-w-[280px]': visibleBars.length > 0 }
      )}
    >
      <DataText as="div" className="font-semibold mb-1.5 text-muted-foreground">
        {formatDurationForWindow(timestamp, windowMs)}
      </DataText>
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
                color: 'var(--color-gray-400)',
                label: fmt(totalOverlay),
              });
            }
            if (totalRest > 0 || segments.length === 0) {
              segments.push({
                value: Math.max(totalRest, 0),
                color: 'var(--color-gray-400)',
                label: fmt(Math.max(totalRest, 0)),
                isDimmed: segments.length > 0,
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

/** ResourceTimeline entity-mark tooltip, reusable by entity Gantt charts. */
export function EntityTooltipContent({
  timestamp,
  windowMs,
  activeMarks,
}: {
  /** Elapsed ms from query start. */
  timestamp: number;
  windowMs: number;
  activeMarks: ActiveMark[];
}) {
  return (
    <div className="overflow-x-hidden px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50">
      <DataText as="div" className="font-semibold mb-1 text-muted-foreground">
        {formatDurationForWindow(timestamp, windowMs)}
      </DataText>
      <ActiveMarksSection marks={activeMarks} />
    </div>
  );
}

export function TooltipContent({
  timestamp,
  series,
  fmt = defaultFormatter,
  windowMs,
  activeMarks,
}: {
  timestamp: number;
  series: TooltipSeries[];
  fmt?: ValueFormatter;
  windowMs: number;
  activeMarks?: ActiveMark[];
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
        isDimmed: base.isDimmed,
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
        fmt={fmt}
        windowMs={windowMs}
        activeMarks={activeMarks}
      />
    );
  }

  return (
    <div className="px-2 py-1.5 bg-popover rounded text-[11px] text-foreground leading-tight shadow-md z-50">
      <DataText as="div" className="font-semibold mb-1 text-muted-foreground">
        {formatDurationForWindow(timestamp, windowMs)}
      </DataText>
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
