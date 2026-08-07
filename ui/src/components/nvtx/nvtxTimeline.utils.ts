// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { NvtxViewportWindow, ZoomRange } from '@quent/utils';

export function nvtxViewportFromZoom(zoomRange: ZoomRange): NvtxViewportWindow {
  return {
    start: zoomRange.start,
    end: Math.max(zoomRange.start, zoomRange.end),
  };
}
