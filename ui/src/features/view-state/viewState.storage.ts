// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ViewStateTab } from './viewState.schema';

export type Awaitable<T> = T | Promise<T>;

export interface ViewStateScope {
  engineId: string;
  queryId: string;
  tab?: ViewStateTab;
}

export interface EncodedViewStateSource {
  readonly id: string;
  load(scope: ViewStateScope): Awaitable<string | null>;
}

export interface EncodedViewStateRepository extends EncodedViewStateSource {
  save(scope: ViewStateScope, encodedState: string): Awaitable<void>;
  remove(scope: ViewStateScope): Awaitable<void>;
}
