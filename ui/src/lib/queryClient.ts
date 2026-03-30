// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { DEFAULT_STALE_TIME } from '@/services/api';
import { QueryClient } from '@tanstack/react-query';

// Create a client for TanStack Query
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: DEFAULT_STALE_TIME, // 5 minutes
      refetchOnWindowFocus: false,
    },
  },
});
