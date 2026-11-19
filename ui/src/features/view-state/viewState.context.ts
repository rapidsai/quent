// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext } from 'react';
import type { ZoomRange } from '@quent/utils';
import type { ViewStateCodecOptions } from './viewState.codec';

export type ViewStateIntakeStatus =
  | { kind: 'idle' }
  | { kind: 'ready' }
  | { kind: 'warning'; message: string }
  | { kind: 'error'; message: string; messages: readonly string[] };

/** `UNAVAILABLE` may succeed later. `UNSAVEABLE` means the current view cannot be encoded. */
export const ENCODED_VIEW_STATE_FAILURE = {
  UNAVAILABLE: 'unavailable',
  UNSAVEABLE: 'unsaveable',
} as const;

export type EncodedViewStateFailureReason =
  (typeof ENCODED_VIEW_STATE_FAILURE)[keyof typeof ENCODED_VIEW_STATE_FAILURE];

export type EncodedViewStateResult =
  | { ok: true; value: string }
  | {
      ok: false;
      reason: EncodedViewStateFailureReason;
      message: string;
      messages: readonly string[];
    };

export interface ViewStateContextValue {
  captureEncodedState(options?: ViewStateCodecOptions): EncodedViewStateResult;
  initialExpandedResourceIds: readonly string[] | null;
  initialZoomRange: ZoomRange | null;
  intakeStatus: ViewStateIntakeStatus;
}

export const ViewStateContext = createContext<ViewStateContextValue | null>(null);

export function useViewState(): ViewStateContextValue | null {
  return useContext(ViewStateContext);
}
