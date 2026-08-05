// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef, useState } from 'react';
import { Pause, Play } from 'lucide-react';
import { cn, formatDurationForWindow } from '@quent/utils';
import {
  useDataFlowEnabled,
  useDataFlowMeta,
  usePlayheadTimeS,
  useSetPlayheadTimeS,
  useSetPlayheadLineTimeMs,
} from '@quent/hooks';

/** Interval between play ticks; each tick advances the playhead by one bin. */
const PLAY_INTERVAL_MS = 100;
const KEYBOARD_STEP_BINS = 1;
const KEYBOARD_FAST_STEP_BINS = 10;

interface DagPlayheadProps {
  className?: string;
}

function formatTimeLabel(timeS: number, windowS: number): string {
  if (timeS === 0) return '0s';
  return formatDurationForWindow(timeS * 1000, Math.max(windowS, Number.EPSILON) * 1000);
}

/**
 * Time slider (playhead) for the DAG data-flow overlay. Plain DOM (not
 * ECharts): a playhead is a point, not a range brush. Writes the playhead
 * atom (rAF-throttled while dragging); `useDataFlowSync` turns playhead
 * changes into per-bin frames. Renders nothing when the feature is
 * unavailable or disabled.
 */
export function DagPlayhead({ className }: DagPlayheadProps) {
  const enabled = useDataFlowEnabled();
  const meta = useDataFlowMeta();
  const playheadTimeS = usePlayheadTimeS();
  const setPlayheadTimeS = useSetPlayheadTimeS();
  const setPlayheadLineTimeMs = useSetPlayheadLineTimeMs();
  const [isPlaying, setIsPlaying] = useState(false);

  const trackRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number | null>(null);
  const pendingClientXRef = useRef<number | null>(null);
  const playheadRef = useRef<number | null>(playheadTimeS);
  playheadRef.current = playheadTimeS;

  const bin = meta?.bin ?? null;

  const clampTime = useCallback(
    (timeS: number): number => {
      if (!bin) return timeS;
      return Math.min(Math.max(timeS, bin.startS), bin.endS);
    },
    [bin]
  );

  const applyClientX = useCallback(
    (clientX: number) => {
      const track = trackRef.current;
      if (!track || !bin) return;
      const rect = track.getBoundingClientRect();
      if (rect.width <= 0) return;
      const t = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
      const timeS = bin.startS + t * (bin.endS - bin.startS);
      setPlayheadTimeS(timeS);
      setPlayheadLineTimeMs(timeS * 1000);
    },
    [bin, setPlayheadTimeS, setPlayheadLineTimeMs]
  );

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      applyClientX(event.clientX);
    },
    [applyClientX]
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
      pendingClientXRef.current = event.clientX;
      if (rafRef.current != null) return;
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        if (pendingClientXRef.current != null) applyClientX(pendingClientXRef.current);
        pendingClientXRef.current = null;
      });
    },
    [applyClientX]
  );

  const handlePointerEnd = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      setPlayheadLineTimeMs(null);
    },
    [setPlayheadLineTimeMs]
  );

  const stepBy = useCallback(
    (bins: number) => {
      if (!bin) return;
      const current = playheadRef.current ?? bin.startS;
      setPlayheadTimeS(clampTime(current + bins * bin.binDurationS));
    },
    [bin, clampTime, setPlayheadTimeS]
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (!bin) return;
      const step = event.shiftKey ? KEYBOARD_FAST_STEP_BINS : KEYBOARD_STEP_BINS;
      switch (event.key) {
        case 'ArrowLeft':
        case 'ArrowDown':
          stepBy(-step);
          break;
        case 'ArrowRight':
        case 'ArrowUp':
          stepBy(step);
          break;
        case 'Home':
          setPlayheadTimeS(bin.startS);
          break;
        case 'End':
          setPlayheadTimeS(bin.endS);
          break;
        default:
          return;
      }
      event.preventDefault();
    },
    [bin, stepBy, setPlayheadTimeS]
  );

  const togglePlay = useCallback(() => {
    if (!bin) return;
    setIsPlaying(playing => {
      if (!playing) {
        // Restart from the window start when play is pressed at the end.
        const current = playheadRef.current ?? bin.startS;
        if (current >= bin.endS) setPlayheadTimeS(bin.startS);
      }
      return !playing;
    });
  }, [bin, setPlayheadTimeS]);

  // Stop playback when the overlay is disabled or the bin metadata goes away:
  // the component stays mounted while rendering null, so a live play interval
  // would otherwise keep advancing the playhead invisibly.
  useEffect(() => {
    if (enabled && bin) return;
    setIsPlaying(false);
    setPlayheadLineTimeMs(null);
  }, [enabled, bin, setPlayheadLineTimeMs]);

  // Advance one bin per tick while playing; stop at the window end.
  useEffect(() => {
    if (!isPlaying || !bin) return;
    const { startS, endS, binDurationS } = bin;
    const id = window.setInterval(() => {
      const current = playheadRef.current ?? startS;
      const next = Math.min(current + binDurationS, endS);
      setPlayheadTimeS(next);
      setPlayheadLineTimeMs(next * 1000);
      if (next >= endS) setIsPlaying(false);
    }, PLAY_INTERVAL_MS);
    return () => window.clearInterval(id);
  }, [isPlaying, bin, setPlayheadTimeS, setPlayheadLineTimeMs]);

  useEffect(() => {
    if (!isPlaying) setPlayheadLineTimeMs(null);
  }, [isPlaying, setPlayheadLineTimeMs]);

  useEffect(() => {
    return () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
      setPlayheadLineTimeMs(null);
    };
  }, [setPlayheadLineTimeMs]);

  if (!enabled || !meta || !bin) return null;

  const windowS = Math.max(bin.endS - bin.startS, Number.EPSILON);
  const timeS = clampTime(playheadTimeS ?? bin.startS);
  const positionPct = ((timeS - bin.startS) / windowS) * 100;
  const currentLabel = formatTimeLabel(timeS, windowS);

  return (
    <div
      className={cn(
        'flex items-center gap-2 border-t bg-card px-3 py-1.5 flex-shrink-0 select-none',
        className
      )}
      data-testid="dag-playhead"
    >
      <button
        onClick={togglePlay}
        aria-label={isPlaying ? 'Pause data flow' : 'Play data flow'}
        title={isPlaying ? 'Pause' : 'Play'}
        className="rounded p-1 hover:bg-muted transition-colors cursor-pointer flex-shrink-0"
      >
        {isPlaying ? (
          <Pause className="h-3 w-3 text-muted-foreground" />
        ) : (
          <Play className="h-3 w-3 text-muted-foreground" />
        )}
      </button>
      <span className="text-[10px] text-muted-foreground tabular-nums flex-shrink-0">
        {formatTimeLabel(bin.startS, windowS)}
      </span>
      <div
        ref={trackRef}
        className="relative h-4 flex-1 min-w-8 cursor-pointer touch-none"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerEnd}
        onPointerCancel={handlePointerEnd}
      >
        <div className="absolute top-1/2 -translate-y-1/2 h-1 w-full rounded-full bg-muted" />
        <div
          className="absolute top-1/2 -translate-y-1/2 h-1 rounded-full bg-primary/40"
          style={{ width: `${positionPct}%` }}
        />
        <div
          role="slider"
          tabIndex={0}
          aria-label="Data flow playhead"
          aria-valuemin={bin.startS}
          aria-valuemax={bin.endS}
          aria-valuenow={timeS}
          aria-valuetext={currentLabel}
          onKeyDown={handleKeyDown}
          className="absolute top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-primary bg-background shadow focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          style={{ left: `${positionPct}%` }}
        />
      </div>
      <span className="text-[10px] text-muted-foreground tabular-nums flex-shrink-0">
        {formatTimeLabel(bin.endS, windowS)}
      </span>
      <span className="text-[10px] font-medium tabular-nums rounded bg-muted px-1.5 py-0.5 flex-shrink-0 min-w-16 text-center">
        {currentLabel}
      </span>
    </div>
  );
}
