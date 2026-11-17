// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef } from 'react';
import {
  browserStorage,
  writeViewAutosave,
  type ViewAutosaveScope,
  type ViewAutosaveSnapshot,
} from './viewAutosave.storage';

interface UseViewAutosaveOptions {
  scope: ViewAutosaveScope;
  /** Changing the tab saves the view the user is leaving. */
  tab: string | undefined;
  /** Autosave stays off until the saved view has been restored. */
  enabled: boolean;
  /** Returns the current view, or null when it cannot be captured right now. */
  capture: () => ViewAutosaveSnapshot | null;
}

/**
 * Remembers the current view without touching the interactive path. Nothing
 * runs while the user pans, zooms, or filters; the view is captured only when
 * the page is hidden or closed, or when the tab or query changes.
 */
export function useViewAutosave({ scope, tab, enabled, capture }: UseViewAutosaveOptions): void {
  const { engineId, queryId } = scope;
  const captureRef = useRef(capture);
  const lastWritten = useRef<string | null>(null);

  // Updated in an effect so an effect cleanup still sees the previous tab's capture.
  useEffect(() => {
    captureRef.current = capture;
  });

  useEffect(() => {
    if (!enabled) {
      return;
    }
    lastWritten.current = null;
    const flush = () => {
      const snapshot = captureRef.current();
      if (!snapshot) {
        return;
      }
      const serialized = JSON.stringify(snapshot);
      if (serialized === lastWritten.current) {
        return;
      }
      if (writeViewAutosave(browserStorage(), { engineId, queryId }, snapshot)) {
        lastWritten.current = serialized;
      }
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        flush();
      }
    };

    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('pagehide', flush);
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('pagehide', flush);
      flush();
    };
    // `tab` is a dependency only so the cleanup above saves the tab being left.
  }, [enabled, engineId, queryId, tab]);
}
