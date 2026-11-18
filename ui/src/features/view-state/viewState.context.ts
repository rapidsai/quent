// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext } from 'react';
import type { ZoomRange } from '@quent/utils';

export type ViewStateIntakeStatus =
  | { kind: 'idle' }
  | { kind: 'ready' }
  | { kind: 'warning'; message: string }
  | { kind: 'error'; message: string; messages: readonly string[] };

export type EncodedViewStateResult =
  { ok: true; value: string } | { ok: false; message: string; messages: readonly string[] };

export interface ViewStateContextValue {
  captureEncodedState(): EncodedViewStateResult;
  initialExpandedResourceIds: readonly string[] | null;
  initialZoomRange: ZoomRange | null;
  intakeStatus: ViewStateIntakeStatus;
}

export const ViewStateContext = createContext<ViewStateContextValue | null>(null);

export function useViewState(): ViewStateContextValue | null {
  return useContext(ViewStateContext);
}
