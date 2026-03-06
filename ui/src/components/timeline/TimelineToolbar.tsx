import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { X, Maximize2, Filter, Settings } from 'lucide-react';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '@/atoms/dag';
import {
  hideTasksAtom,
  zoomRangeAtom,
  debouncedZoomRangeAtom,
  timelineDensityAtom,
} from '@/atoms/timeline';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';

export function TimelineToolbar({ durationSeconds }: { durationSeconds: number }) {
  const operatorLabel = useAtomValue(selectedOperatorLabelAtom);
  const setSelectedNodeIds = useSetAtom(selectedNodeIdsAtom);
  const setSelectedOperatorLabel = useSetAtom(selectedOperatorLabelAtom);
  const [hideTasks, setHideTasks] = useAtom(hideTasksAtom);
  const [density, setDensity] = useAtom(timelineDensityAtom);
  const setZoomRange = useSetAtom(zoomRangeAtom);
  const setDebouncedZoomRange = useSetAtom(debouncedZoomRangeAtom);

  const clearOperator = () => {
    setSelectedNodeIds(new Set());
    setSelectedOperatorLabel(null);
  };

  const resetZoom = () => {
    const full = { start: 0, end: durationSeconds };
    setZoomRange(full);
    setDebouncedZoomRange(full);
  };

  return (
    <div className="flex items-center gap-4 px-3 py-1 border-b border-border text-xs text-muted-foreground shrink-0">
      {/* Operator filter */}
      <div className="flex items-center gap-1.5">
        <Filter className="h-3 w-3" />
        {operatorLabel ? (
          <span className="inline-flex items-center gap-1 rounded-sm bg-primary/15 text-primary px-1.5 py-0.5 font-medium">
            {operatorLabel}
            <button
              onClick={clearOperator}
              className="rounded-sm hover:bg-primary/20 p-0.5 -mr-0.5 transition-colors"
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </span>
        ) : (
          <span>No filters</span>
        )}
      </div>

      <div className="flex-1" />

      {/* Zoom reset */}
      <button
        onClick={resetZoom}
        className="inline-flex items-center gap-1 rounded-sm px-1.5 py-0.5 hover:bg-accent hover:text-accent-foreground transition-colors"
        title="Reset zoom"
      >
        <Maximize2 className="h-3 w-3" />
        <span>Reset zoom</span>
      </button>

      <div className="h-3 w-px bg-border" />

      {/* Settings popover */}
      <Popover>
        <PopoverTrigger asChild>
          <button
            className="inline-flex items-center rounded-sm p-0.5 hover:bg-accent hover:text-accent-foreground transition-colors"
            title="Timeline settings"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>
        </PopoverTrigger>
        <PopoverContent className="text-xs space-y-4">
          <div className="space-y-2">
            <span className="text-muted-foreground font-medium">Annotations</span>
            <label className="flex items-center gap-2 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={hideTasks}
                onChange={e => setHideTasks(e.target.checked)}
                className="h-3 w-3 rounded-sm accent-primary cursor-pointer"
              />
              <span>Hide tasks</span>
            </label>
          </div>

          <div className="space-y-2">
            <span className="text-muted-foreground font-medium">Row height</span>
            <div className="inline-flex rounded-md border border-border overflow-hidden w-full">
              {(['comfortable', 'compact'] as const).map(option => (
                <button
                  key={option}
                  onClick={() => setDensity(option)}
                  className={`flex-1 px-2.5 py-1 text-xs capitalize transition-colors ${
                    density === option
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-transparent hover:bg-accent text-muted-foreground'
                  }`}
                >
                  {option}
                </button>
              ))}
            </div>
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}
