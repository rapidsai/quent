// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

type PortPair = { sourcePortId?: string; targetPortId?: string };

/**
 * A pipe is identified by its port pair, so parallel pipes between the same operators differ.
 * Edges can lack port IDs, and those never match anything.
 */
export function isSamePipe(
  a: PortPair | null | undefined,
  b: PortPair | null | undefined
): boolean {
  return (
    !!a &&
    !!b &&
    a.sourcePortId !== undefined &&
    a.targetPortId !== undefined &&
    a.sourcePortId === b.sourcePortId &&
    a.targetPortId === b.targetPortId
  );
}
