// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import type { ResolvedViewState } from './useResolvedViewState';
import { showViewStateErrorToasts } from './viewState.toasts';

const FALLBACK_MESSAGE = 'The saved view could not be loaded.';

export function ViewStateResolutionToast({ resolution }: { resolution: ResolvedViewState }) {
  useEffect(() => {
    if (resolution.status !== 'error') {
      return;
    }
    console.error(`Failed to load view state from "${resolution.sourceId}":`, resolution.error);
    showViewStateErrorToasts({
      id: 'view-state-resolution',
      title: 'Could not restore saved view',
      messages: [
        resolution.error instanceof Error && resolution.error.message
          ? resolution.error.message
          : FALLBACK_MESSAGE,
      ],
    });
  }, [resolution]);

  return null;
}
