// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EncodedViewStateSource } from './viewState.storage';

export const URL_VIEW_STATE_SOURCE_ID = 'url';

export function createUrlViewStateSource(encodedState: string | undefined): EncodedViewStateSource {
  return {
    id: URL_VIEW_STATE_SOURCE_ID,
    load: () => encodedState ?? null,
  };
}
