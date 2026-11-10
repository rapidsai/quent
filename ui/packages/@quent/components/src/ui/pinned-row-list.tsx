// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type Ref,
  type RefObject,
} from 'react';
import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import { restrictToParentElement, restrictToVerticalAxis } from '@dnd-kit/modifiers';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { defaultRangeExtractor, useVirtualizer, type Range } from '@tanstack/react-virtual';
import { cn } from '@quent/utils';

// Presses on these start the control, not a drag. Dropdown lists render in a portal but their
// events still bubble through React, so match them too.
const ROW_CONTROL_SELECTOR =
  'button, a, input, select, textarea, [role="button"], [role="combobox"], [role="listbox"], [role="option"], [role="menuitem"], [data-radix-popper-content-wrapper]';

// Rows drawn above and below the visible ones, so scrolling does not show blank space.
const OVERSCAN_ROWS = 8;

// How far the pointer must move before a press becomes a drag, so plain clicks stay clicks.
const DRAG_START_DISTANCE_PX = 5;

// The normal pointer sensor, except a press that lands on a control inside the drag area is left
// alone. The drag area itself is marked as a button, so it does not count as one of those.
class RowPointerSensor extends PointerSensor {
  static activators = [
    {
      eventName: 'onPointerDown' as const,
      handler: ({ nativeEvent, currentTarget }: ReactPointerEvent) => {
        if (!nativeEvent.isPrimary || nativeEvent.button !== 0) {
          return false;
        }
        const control =
          nativeEvent.target instanceof Element
            ? nativeEvent.target.closest(ROW_CONTROL_SELECTOR)
            : null;
        return control === null || control === currentTarget;
      },
    },
  ];
}

/**
 * What a row's info cell needs to act as the drag area: spread `attributes` and `listeners` on
 * it, attach `ref`, and use `isDragging` to pick the cursor.
 */
export interface PinnedRowDragArea {
  ref: Ref<HTMLElement>;
  attributes: ReturnType<typeof useSortable>['attributes'];
  listeners: ReturnType<typeof useSortable>['listeners'];
  isDragging: boolean;
}

interface PinnedRowProps {
  id: string;
  index: number;
  count: number;
  top: number;
  measureRef: (element: HTMLElement | null) => void;
  isSortable: boolean;
  minHeight: number;
  paddingLeft: number;
  children: (dragArea: PinnedRowDragArea | undefined) => ReactNode;
}

function PinnedRow({
  id,
  index,
  count,
  top,
  measureRef,
  isSortable,
  minHeight,
  paddingLeft,
  children,
}: PinnedRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, disabled: !isSortable });
  const setRefs = useCallback(
    (element: HTMLElement | null) => {
      setNodeRef(element);
      measureRef(element);
    },
    [setNodeRef, measureRef]
  );
  // The row's place in the list is set with `top`, so the drag transform is free to move it.
  const style: CSSProperties = {
    position: 'absolute',
    top: `${top}px`,
    left: 0,
    right: 0,
    minHeight: `${minHeight}px`,
    paddingLeft: `${paddingLeft}px`,
    transform: CSS.Translate.toString(transform),
    transition,
  };
  return (
    <div
      ref={setRefs}
      role="listitem"
      data-index={index}
      aria-posinset={index + 1}
      aria-setsize={count}
      className={cn(
        'flex items-center text-foreground',
        isSortable && 'hover:bg-secondary/10',
        isDragging && 'z-10 bg-background shadow-md'
      )}
      style={style}
    >
      {/* Only the info cell picks the row up, so the timelines keep their normal cursor. */}
      {children(
        isSortable ? { ref: setActivatorNodeRef, attributes, listeners, isDragging } : undefined
      )}
    </div>
  );
}

interface PinnedRowListProps<T extends { id: string }> {
  items: T[];
  rowMinHeight: number;
  rowPaddingLeft: number;
  /** The element that scrolls the list. Only the rows near its viewport are drawn. */
  scrollRef: RefObject<HTMLElement | null>;
  renderRow: (item: T, dragArea: PinnedRowDragArea | undefined) => ReactNode;
  /** Called when a row is dropped on another; without it the rows cannot be dragged. */
  onReorder?: (activeId: string, overId: string) => void;
}

/**
 * The pinned rows as a list, which the user can rearrange by dragging the info cell (name and
 * type), or with the keyboard: focus that cell, press Space, use the arrow keys, then press Space again.
 * Only the rows near the visible area are drawn, so hundreds of pins stay fast.
 */
export function PinnedRowList<T extends { id: string }>({
  items,
  rowMinHeight,
  rowPaddingLeft,
  scrollRef,
  renderRow,
  onReorder,
}: PinnedRowListProps<T>) {
  const sensors = useSensors(
    useSensor(RowPointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE_PX } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const listRef = useRef<HTMLDivElement>(null);
  // How far down the scroller the list starts, because the header sits above it.
  const [scrollMargin, setScrollMargin] = useState(0);
  useLayoutEffect(() => {
    setScrollMargin(listRef.current?.offsetTop ?? 0);
  }, [items.length]);

  // The row being dragged stays drawn even if scrolling carries it out of view.
  const [activeId, setActiveId] = useState<string | null>(null);
  const activeIndex = activeId === null ? -1 : items.findIndex(item => item.id === activeId);

  const rangeExtractor = useCallback(
    (range: Range) => {
      const indexes = defaultRangeExtractor(range);
      if (activeIndex < 0 || indexes.includes(activeIndex)) {
        return indexes;
      }
      return [...indexes, activeIndex].sort((a, b) => a - b);
    },
    [activeIndex]
  );

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowMinHeight,
    overscan: OVERSCAN_ROWS,
    scrollMargin,
    getItemKey: index => items[index]?.id ?? index,
    rangeExtractor,
  });

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    setActiveId(null);
    if (over && active.id !== over.id) {
      onReorder?.(String(active.id), String(over.id));
    }
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      modifiers={[restrictToVerticalAxis, restrictToParentElement]}
      onDragStart={({ active }) => setActiveId(String(active.id))}
      onDragCancel={() => setActiveId(null)}
      onDragEnd={handleDragEnd}
    >
      <SortableContext items={items.map(item => item.id)} strategy={verticalListSortingStrategy}>
        <div
          ref={listRef}
          role="list"
          aria-label="Pinned rows list"
          // Lets the panel size itself to the rows before they have all been drawn.
          data-count={items.length}
          data-row-height={rowMinHeight}
          style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}
        >
          {virtualizer.getVirtualItems().map(virtualRow => {
            const item = items[virtualRow.index];
            if (!item) {
              return null;
            }
            return (
              <PinnedRow
                key={item.id}
                id={item.id}
                index={virtualRow.index}
                count={items.length}
                top={virtualRow.start - scrollMargin}
                measureRef={virtualizer.measureElement}
                isSortable={onReorder !== undefined}
                minHeight={rowMinHeight}
                paddingLeft={rowPaddingLeft}
              >
                {dragArea => renderRow(item, dragArea)}
              </PinnedRow>
            );
          })}
        </div>
      </SortableContext>
    </DndContext>
  );
}
