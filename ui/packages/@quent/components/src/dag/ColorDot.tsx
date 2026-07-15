// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/** Small square color swatch used as an inline legend marker. */
export const ColorDot = ({ color }: { color: string }) => (
  <span className="inline-block h-2 w-2 rounded-sm shrink-0" style={{ backgroundColor: color }} />
);
