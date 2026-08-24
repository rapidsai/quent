// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext } from 'react';
import type { ZoomRange } from '@quent/utils';

export type DeepLinkIntakeStatus =
  | { kind: 'idle' }
  | { kind: 'ready' }
  | { kind: 'warning'; message: string }
  | { kind: 'error'; message: string };

export type CopyLinkResult = { ok: true; url: string } | { ok: false; message: string };

export interface DeepLinkContextValue {
  copyLink(): Promise<CopyLinkResult>;
  initialExpandedResourceIds: readonly string[] | null;
  initialZoomRange: ZoomRange | null;
  intakeStatus: DeepLinkIntakeStatus;
}

export const DeepLinkContext = createContext<DeepLinkContextValue | null>(null);

export function useDeepLink(): DeepLinkContextValue | null {
  return useContext(DeepLinkContext);
}
