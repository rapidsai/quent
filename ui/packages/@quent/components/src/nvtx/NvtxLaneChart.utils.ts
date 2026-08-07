// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { formatDuration } from '@quent/utils';

/** Convert query-relative seconds from the NVTX API to the millisecond chart axis. */
export function nvtxRelativeSecondsToMs(seconds: number): number {
  return seconds * 1_000;
}

/** Format a duration supplied by the NVTX API in seconds. */
export function formatNvtxDuration(seconds: number): string {
  return formatDuration(nvtxRelativeSecondsToMs(seconds));
}
