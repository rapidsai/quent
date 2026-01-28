import { Skeleton } from '@/components/ui/skeleton';
import { DEFAULT_TIMELINE_HEIGHT } from './types';

type TimelineSkeletonProps = {
  height?: number;
};

export function TimelineSkeleton({ height = DEFAULT_TIMELINE_HEIGHT }: TimelineSkeletonProps) {
  return (
    <div className="relative w-full" style={{ height: `${height}px` }}>
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
          {Array.from({ length: 24 }).map((_, i) => (
            <Skeleton
              key={i}
              className="mx-0.5 flex-1 rounded-t-sm"
              style={{
                height: `${30 + Math.sin(i * 0.5) * 20 + Math.random() * 25}%`,
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
