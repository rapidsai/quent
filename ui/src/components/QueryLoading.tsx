// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Loader2 } from 'lucide-react';

export function QueryLoading() {
  return (
    <div
      role="status"
      aria-label="Loading query"
      className="flex min-h-[calc(100vh-4rem)] w-full items-center justify-center gap-2 text-muted-foreground"
    >
      <Loader2 className="size-5 animate-spin" aria-hidden="true" />
      <span>Loading query...</span>
    </div>
  );
}
