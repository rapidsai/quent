// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { queryOptions, useQuery } from '@tanstack/react-query';
import { fetchListEngines } from './api';
import { DEFAULT_STALE_TIME } from './constants';

export const enginesQueryOptions = (options?: { staleTime?: number }) =>
  queryOptions({
    queryKey: ['list_engines'],
    queryFn: fetchListEngines,
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });

export const useEngines = (options?: { staleTime?: number }) =>
  useQuery(enginesQueryOptions(options));
