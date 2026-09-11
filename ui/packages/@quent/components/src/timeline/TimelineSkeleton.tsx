// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Skeleton } from '../ui/skeleton';

const WAVEFORM_BAR_HEIGHTS = Array.from(
  { length: 24 },
  (_, index) => 30 + Math.sin(index * 0.5) * 20 + ((index * 17) % 25)
);

/** Animated skeleton placeholder rendered while timeline data loads. */
export function TimelineSkeleton() {
  return (
    <div className="relative w-full h-full">
      {/* Chart area background */}
      <div
        className="absolute rounded-sm bg-muted/30"
        style={{
          left: 40,
          right: 10,
          top: 10,
          bottom: 30,
        }}
      >
        {/* Simulated waveform skeleton */}
        <div className="absolute inset-0 flex items-end overflow-hidden px-2 pb-2">
          {WAVEFORM_BAR_HEIGHTS.map((height, i) => (
            <Skeleton
              key={i}
              className="mx-0.5 flex-1 rounded-t-sm"
              style={{
                height: `${height}%`,
                animationDelay: `${i * 50}ms`,
              }}
            />
          ))}
        </div>
      </div>

      {/* Y-axis skeleton */}
      <div className="absolute left-0 top-2.5 flex h-[calc(100%-40px)] flex-col justify-between">
        <Skeleton className="h-3 w-8" />
        <Skeleton className="h-3 w-6" />
      </div>

      {/* X-axis skeleton */}
      <div className="absolute bottom-1.5 left-10 right-2.5 flex justify-between">
        <Skeleton className="h-3 w-12" />
        <Skeleton className="h-3 w-12" />
        <Skeleton className="h-3 w-12" />
      </div>
    </div>
  );
}
