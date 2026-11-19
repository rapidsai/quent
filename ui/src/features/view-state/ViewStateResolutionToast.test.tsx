// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { toast } from '@quent/components';
import { render } from '@/test/test-utils';
import { ViewStateResolutionToast } from './ViewStateResolutionToast';

describe('ViewStateResolutionToast', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows an error toast when a source fails to load', () => {
    const toastSpy = vi.spyOn(toast, 'add');
    vi.spyOn(console, 'error').mockImplementation(() => undefined);

    render(
      <ViewStateResolutionToast
        resolution={{
          status: 'error',
          encodedState: null,
          sourceId: 'local-storage',
          error: new Error('storage exploded'),
        }}
      />
    );

    expect(toastSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'error',
        title: 'Could not restore saved view',
        description: 'storage exploded',
      })
    );
  });

  it('stays quiet when state resolved without a failure', () => {
    const toastSpy = vi.spyOn(toast, 'add');

    render(
      <ViewStateResolutionToast
        resolution={{ status: 'ready', encodedState: null, sourceId: null }}
      />
    );

    expect(toastSpy).not.toHaveBeenCalled();
  });
});
