// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export { captureViewExtras, hydrateViewExtras } from './viewExtras';
export { useViewAutosave } from './useViewAutosave';
export { useRestoredView } from './useRestoredView';
export {
  browserStorage,
  clearViewAutosave,
  readViewAutosave,
  writeViewAutosave,
  type ViewAutosaveScope,
  type ViewAutosaveSnapshot,
} from './viewAutosave.storage';
export type { ViewAutosaveRecord, ViewExtras } from './viewAutosave.schema';
