// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Badge, DataText, nvtxKindLabel, rgbHex, thinScrollbarClass } from '@quent/components';
import { useNvtxSpanDetail } from '@quent/client';
import { cn, formatDuration } from '@quent/utils';
import type { NvtxPayloadItem, NvtxSpanSummary } from '@quent/utils';

interface NvtxDetailPanelProps {
  contextId: string | null;
  spanId: number | null;
  queryStartUnixNs: bigint;
  onSelectSpan: (spanId: number) => void;
}

/** An instance is flagged as a duration outlier past this multiple of the peer average. */
const OUTLIER_DURATION_MULTIPLE = 2;

function payloadLabel(payload: NvtxPayloadItem): string {
  switch (payload.type) {
    case 'unsigned_int64':
      return `Payload (uint64): ${payload.value}`;
    case 'int64':
      return `Payload (int64): ${payload.value}`;
    case 'double':
      return `Payload (double): ${payload.value}`;
    case 'unsigned_int32':
      return `Payload (uint32): ${payload.value}`;
    case 'int32':
      return `Payload (int32): ${payload.value}`;
    case 'float':
      return `Payload (float): ${payload.value}`;
    case 'pointer':
      return `Payload (pointer): ${payload.value}`;
  }
}

function SpanSummaryRow({
  summary,
  onSelectSpan,
}: {
  summary: NvtxSpanSummary;
  onSelectSpan: (spanId: number) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onSelectSpan(summary.span_id)}
      className="flex w-full items-center justify-between gap-2 rounded border bg-card px-2 py-1 text-left text-xs transition-colors hover:bg-accent"
    >
      <DataText className="min-w-0 flex-1 truncate">{summary.message}</DataText>
      <DataText className="shrink-0 tabular-nums text-muted-foreground">
        {summary.duration != null ? formatDuration(summary.duration * 1_000) : '(open)'}
      </DataText>
    </button>
  );
}

export function NvtxDetailPanel({
  contextId,
  spanId,
  queryStartUnixNs,
  onSelectSpan,
}: NvtxDetailPanelProps) {
  const { data: detail, isLoading } = useNvtxSpanDetail(contextId ?? '', spanId, queryStartUnixNs);

  if (spanId == null) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        Select an NVTX range to view its details.
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        Loading range details…
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-sm text-muted-foreground">
        This range could not be found.
      </div>
    );
  }

  const durationMs = detail.duration != null ? detail.duration * 1_000 : null;
  const statistics = detail.statistics;
  const avgDurationMs = statistics ? statistics.avg_duration * 1_000 : null;
  const isOutlier =
    durationMs != null &&
    avgDurationMs != null &&
    avgDurationMs > 0 &&
    durationMs > avgDurationMs * OUTLIER_DURATION_MULTIPLE;

  return (
    <div className={cn('flex h-full min-h-0 flex-col overflow-auto p-3', thinScrollbarClass)}>
      {/* Header: name, color dot, kind badge, domain/category chips */}
      <div className="flex items-center gap-2">
        <span
          aria-hidden
          className="inline-block size-2.5 shrink-0 rounded-full"
          style={{ backgroundColor: rgbHex(detail.color) }}
        />
        <DataText className="min-w-0 flex-1 truncate text-sm font-medium">
          {detail.message}
        </DataText>
      </div>
      <div className="mt-1.5 flex flex-wrap items-center gap-1">
        <Badge variant="outline">{nvtxKindLabel(detail.kind)}</Badge>
        <Badge variant="outline">{detail.domain_name}</Badge>
        {detail.category_name && <Badge variant="outline">{detail.category_name}</Badge>}
        {detail.incomplete && <Badge variant="destructive">incomplete</Badge>}
      </div>

      {/* Summary strip */}
      <div className="mt-3 rounded border bg-muted/30 px-2 py-1.5 text-xs">
        <div className="flex items-center justify-between gap-2">
          <span className="text-muted-foreground">Duration</span>
          <DataText
            className={cn(
              'tabular-nums font-medium',
              isOutlier && 'text-orange-500 dark:text-orange-400'
            )}
          >
            {durationMs != null ? formatDuration(durationMs) : '(open)'}
          </DataText>
        </div>
        {detail.thread_name && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Thread</span>
            <DataText className="truncate">{detail.thread_name}</DataText>
          </div>
        )}
        {detail.payload && (
          <div className="mt-0.5 flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Payload</span>
            <DataText className="truncate tabular-nums">{payloadLabel(detail.payload)}</DataText>
          </div>
        )}
      </div>

      {/* Peer comparison */}
      {statistics && (
        <div className="mt-3">
          <div className="text-xs font-medium text-muted-foreground">
            Compared to {statistics.count.toString()} occurrence
            {statistics.count === 1n ? '' : 's'} of &ldquo;{statistics.message}&rdquo;
          </div>
          <div className="mt-1 space-y-0.5 rounded border bg-card px-2 py-1.5 text-xs">
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted-foreground">Average</span>
              <DataText className="tabular-nums">
                {formatDuration(statistics.avg_duration * 1_000)}
              </DataText>
            </div>
            {statistics.min_duration != null && (
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">Min</span>
                <DataText className="tabular-nums">
                  {formatDuration(statistics.min_duration * 1_000)}
                </DataText>
              </div>
            )}
            {statistics.max_duration != null && (
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">Max</span>
                <DataText className="tabular-nums">
                  {formatDuration(statistics.max_duration * 1_000)}
                </DataText>
              </div>
            )}
          </div>
          {isOutlier && (
            <div className="mt-1 text-xs text-orange-500 dark:text-orange-400">
              This instance took more than {OUTLIER_DURATION_MULTIPLE}&times; the average duration.
            </div>
          )}
        </div>
      )}

      {/* Nesting hierarchy */}
      {(detail.ancestors.length > 0 || detail.children.length > 0) && (
        <div className="mt-3">
          {detail.ancestors.length > 0 && (
            <>
              <div className="text-xs font-medium text-muted-foreground">Ancestors</div>
              <div className="mt-1 space-y-1">
                {detail.ancestors.map(ancestor => (
                  <SpanSummaryRow
                    key={ancestor.span_id}
                    summary={ancestor}
                    onSelectSpan={onSelectSpan}
                  />
                ))}
              </div>
            </>
          )}
          {detail.children.length > 0 && (
            <>
              <div
                className={cn(
                  'text-xs font-medium text-muted-foreground',
                  detail.ancestors.length > 0 && 'mt-2'
                )}
              >
                Children
              </div>
              <div className="mt-1 space-y-1">
                {detail.children.map(child => (
                  <SpanSummaryRow key={child.span_id} summary={child} onSelectSpan={onSelectSpan} />
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
