// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

export type DropPosition = 'before' | 'after';

interface DropIndicator {
  id: string;
  position: DropPosition;
}

interface UseColumnDragDropOptions {
  onDropCommit: (draggedId: string, targetId: string, position: DropPosition) => void;
  createDragPreview?: (e: React.DragEvent<HTMLElement>, itemId: string) => void | (() => void);
}

export function useColumnDragDrop({ onDropCommit, createDragPreview }: UseColumnDragDropOptions) {
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dropIndicator, setDropIndicator] = useState<DropIndicator | null>(null);
  const draggedIdRef = useRef<string | null>(null);
  const cleanupDragPreviewRef = useRef<(() => void) | null>(null);

  const clearDragPreview = useCallback(() => {
    cleanupDragPreviewRef.current?.();
    cleanupDragPreviewRef.current = null;
  }, []);

  const resetDragState = useCallback(() => {
    draggedIdRef.current = null;
    setDraggedId(null);
    setDropIndicator(null);
    clearDragPreview();
  }, [clearDragPreview]);

  const getDropPosition = useCallback((e: React.DragEvent<HTMLElement>): DropPosition => {
    const rect = e.currentTarget.getBoundingClientRect();
    return e.clientX - rect.left < rect.width / 2 ? 'before' : 'after';
  }, []);

  const handleDragStart = useCallback(
    (e: React.DragEvent<HTMLElement>, itemId: string) => {
      clearDragPreview();
      draggedIdRef.current = itemId;
      setDropIndicator(null);
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', itemId);

      const cleanup = createDragPreview?.(e, itemId);
      cleanupDragPreviewRef.current = typeof cleanup === 'function' ? cleanup : null;
    },
    [clearDragPreview, createDragPreview]
  );

  const handleDragOver = useCallback(
    (e: React.DragEvent<HTMLElement>, targetId: string) => {
      e.preventDefault();
      const activeDraggedId = draggedIdRef.current;
      if (activeDraggedId == null || activeDraggedId === targetId) return;
      // Avoid re-rendering in dragstart, which can cancel native DnD in some browsers.
      setDraggedId(prev => (prev === activeDraggedId ? prev : activeDraggedId));
      const position = getDropPosition(e);
      setDropIndicator(prev =>
        prev?.id === targetId && prev.position === position ? prev : { id: targetId, position }
      );
      e.dataTransfer.dropEffect = 'move';
    },
    [getDropPosition]
  );

  const handleDragLeave = useCallback((e: React.DragEvent<HTMLElement>, targetId: string) => {
    const related = e.relatedTarget as Node | null;
    if (related && e.currentTarget.contains(related)) return;
    setDropIndicator(prev => (prev?.id === targetId ? null : prev));
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent<HTMLElement>, targetId: string) => {
      e.preventDefault();
      const activeDraggedId = draggedIdRef.current;
      if (activeDraggedId == null || activeDraggedId === targetId) {
        resetDragState();
        return;
      }
      const position = dropIndicator?.id === targetId ? dropIndicator.position : getDropPosition(e);
      onDropCommit(activeDraggedId, targetId, position);
      resetDragState();
    },
    [dropIndicator, getDropPosition, onDropCommit, resetDragState]
  );

  useEffect(() => {
    if (draggedId == null) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      resetDragState();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [draggedId, resetDragState]);

  useEffect(() => clearDragPreview, [clearDragPreview]);

  const getDropTargetPosition = useCallback(
    (itemId: string): DropPosition | undefined =>
      dropIndicator?.id === itemId ? dropIndicator.position : undefined,
    [dropIndicator]
  );

  return useMemo(
    () => ({
      draggedId,
      getDropTargetPosition,
      handleDragStart,
      handleDragOver,
      handleDragLeave,
      handleDrop,
      handleDragEnd: resetDragState,
    }),
    [
      draggedId,
      getDropTargetPosition,
      handleDragStart,
      handleDragOver,
      handleDragLeave,
      handleDrop,
      resetDragState,
    ]
  );
}
