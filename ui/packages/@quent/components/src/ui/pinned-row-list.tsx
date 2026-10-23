// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { CSSProperties, PointerEvent as ReactPointerEvent, ReactNode, Ref } from 'react';
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
import { cn } from '@quent/utils';
import { ThinScroll } from './thin-scroll';

// Presses on these start the control, not a drag. Dropdown lists render in a portal but their
// events still bubble through React, so match them too.
const ROW_CONTROL_SELECTOR =
  'button, a, input, select, textarea, [role="button"], [role="combobox"], [role="listbox"], [role="option"], [role="menuitem"], [data-radix-popper-content-wrapper]';

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
  isSortable: boolean;
  minHeight: number;
  paddingLeft: number;
  children: (dragArea: PinnedRowDragArea | undefined) => ReactNode;
}

function PinnedRow({ id, isSortable, minHeight, paddingLeft, children }: PinnedRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, disabled: !isSortable });
  const style: CSSProperties = {
    minHeight: `${minHeight}px`,
    paddingLeft: `${paddingLeft}px`,
    transform: CSS.Translate.toString(transform),
    transition,
  };
  return (
    <div
      ref={setNodeRef}
      role="listitem"
      className={cn(
        'relative flex items-center text-foreground',
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
  maxHeight: number;
  rowMinHeight: number;
  rowPaddingLeft: number;
  renderRow: (item: T, dragArea: PinnedRowDragArea | undefined) => ReactNode;
  /** Called when a row is dropped on another; without it the rows cannot be dragged. */
  onReorder?: (activeId: string, overId: string) => void;
}

/**
 * The pinned rows as a scrolling list the user can rearrange by dragging the info cell (name and
 * type), or with the keyboard: focus that cell, press Space, use the arrow keys, then press Space again.
 */
export function PinnedRowList<T extends { id: string }>({
  items,
  maxHeight,
  rowMinHeight,
  rowPaddingLeft,
  renderRow,
  onReorder,
}: PinnedRowListProps<T>) {
  const sensors = useSensors(
    useSensor(RowPointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE_PX } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (over && active.id !== over.id) {
      onReorder?.(String(active.id), String(over.id));
    }
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      modifiers={[restrictToVerticalAxis, restrictToParentElement]}
      onDragEnd={handleDragEnd}
    >
      <SortableContext items={items.map(item => item.id)} strategy={verticalListSortingStrategy}>
        <ThinScroll
          role="list"
          aria-label="Pinned rows list"
          className="overscroll-none"
          style={{ maxHeight: `${maxHeight}px` }}
        >
          {items.map(item => (
            <PinnedRow
              key={item.id}
              id={item.id}
              isSortable={onReorder !== undefined}
              minHeight={rowMinHeight}
              paddingLeft={rowPaddingLeft}
            >
              {dragArea => renderRow(item, dragArea)}
            </PinnedRow>
          ))}
        </ThinScroll>
      </SortableContext>
    </DndContext>
  );
}
