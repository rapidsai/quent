// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export type PlotMode = 'simple' | 'advanced';

const storageKey = 'quent-bench-plot-mode';
const noopStorageKey = 'quent-bench-include-noop';
const discardedStorageKey = 'quent-bench-include-discarded';

export const plotPreference = $state<{ mode: PlotMode; includeNoop: boolean; includeDiscarded: boolean }>({
  mode: 'advanced', includeNoop: true, includeDiscarded: true,
});

export function loadPlotPreference(): void {
  try {
    const stored = localStorage.getItem(storageKey);
    if (stored === 'simple' || stored === 'advanced') plotPreference.mode = stored;
    const includeNoop = localStorage.getItem(noopStorageKey);
    if (includeNoop === 'true' || includeNoop === 'false') plotPreference.includeNoop = includeNoop === 'true';
    const includeDiscarded = localStorage.getItem(discardedStorageKey);
    if (includeDiscarded === 'true' || includeDiscarded === 'false') plotPreference.includeDiscarded = includeDiscarded === 'true';
  } catch {
    // Browser storage can be unavailable; the page still works with its default.
  }
}

export function setPlotMode(mode: PlotMode): void {
  plotPreference.mode = mode;
  try {
    localStorage.setItem(storageKey, mode);
  } catch {
    // Keep the selection for this page when browser storage is unavailable.
  }
}

export function setIncludeNoop(includeNoop: boolean): void {
  plotPreference.includeNoop = includeNoop;
  try {
    localStorage.setItem(noopStorageKey, String(includeNoop));
  } catch {
    // Keep the selection for this page when browser storage is unavailable.
  }
}

export function setIncludeDiscarded(includeDiscarded: boolean): void {
  plotPreference.includeDiscarded = includeDiscarded;
  try {
    localStorage.setItem(discardedStorageKey, String(includeDiscarded));
  } catch {
    // Keep the selection for this page when browser storage is unavailable.
  }
}
