// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect } from 'react';
import { Provider as JotaiProvider, useAtomValue, useSetAtom } from 'jotai';
import {
  useDebouncedZoomRange,
  useHydrateTimelineAtoms,
  useSetDebouncedZoomRange,
  useSetZoomRange,
  useZoomRange,
} from '@quent/hooks';
import { toast, Toaster } from '@quent/components';
import { render, screen, waitFor, userEvent } from '@/test/test-utils';
import { expandedIdsAtom } from '@/atoms/resourceTree';
import { CopyLinkButton } from './CopyLinkButton';
import { DeepLinkBoundary } from './DeepLinkBoundary';
import { decodeDeepLinkState, encodeDeepLinkState } from './deepLink.codec';
import { DEEP_LINK_NAV_SLOT_ID } from './deepLink.constants';
import { useDeepLink } from './deepLink.context';

const RESOURCE_A_ID = '01a025ff-ea8b-7881-9d31-72a275872c9d';
const RESOURCE_B_ID = '01a025ff-ea8b-7881-9d31-72a275872c9e';

function ViewportProbe() {
  const immediate = useZoomRange();
  const debounced = useDebouncedZoomRange();
  return <output data-testid="viewport">{JSON.stringify({ immediate, debounced })}</output>;
}

function IntakeStatusProbe() {
  const deepLink = useDeepLink();
  return <output data-testid="intake-status">{deepLink?.intakeStatus.kind}</output>;
}

function ExpandedRowsProbe() {
  const expandedIds = useAtomValue(expandedIdsAtom);
  return <output data-testid="expanded-rows">{JSON.stringify([...expandedIds].sort())}</output>;
}

function SeedViewport({ start, end }: { start: number; end: number }) {
  const setImmediate = useSetZoomRange();
  const setDebounced = useSetDebouncedZoomRange();

  useLayoutEffect(() => {
    setImmediate({ start, end });
    setDebounced({ start, end });
  }, [end, setDebounced, setImmediate, start]);
  return null;
}

function SeedExpandedRows({ ids }: { ids: string[] }) {
  const setExpandedIds = useSetAtom(expandedIdsAtom);

  useLayoutEffect(() => {
    setExpandedIds(new Set(ids));
  }, [ids, setExpandedIds]);
  return null;
}

function HydrateTimelineDuringRender() {
  useHydrateTimelineAtoms({
    zoomRange: { start: 0, end: 100 },
    debouncedZoomRange: { start: 0, end: 100 },
    startTimeMs: 0,
  });
  return null;
}

describe('DeepLinkBoundary', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates timeline viewport and expanded rows before rendering children', () => {
    const encoded = encodeDeepLinkState({
      zoomRange: { start: 10, end: 40 },
      expandedResourceIds: [RESOURCE_B_ID, RESOURCE_A_ID],
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) return;

    render(
      <JotaiProvider>
        <DeepLinkBoundary durationSeconds={100} encodedState={encoded.value} isQueryReady>
          <ViewportProbe />
          <ExpandedRowsProbe />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.getByTestId('viewport')).toHaveTextContent(
      JSON.stringify({
        immediate: { start: 10, end: 40 },
        debounced: { start: 10, end: 40 },
      })
    );
    expect(screen.getByTestId('expanded-rows')).toHaveTextContent(
      JSON.stringify([RESOURCE_A_ID, RESOURCE_B_ID])
    );
  });

  it('shows a spinner while waiting for the query to be ready', () => {
    const encoded = encodeDeepLinkState({
      zoomRange: { start: 10, end: 40 },
    });
    expect(encoded.ok).toBe(true);
    if (!encoded.ok) return;

    const { rerender } = render(
      <JotaiProvider>
        <DeepLinkBoundary durationSeconds={0} encodedState={encoded.value} isQueryReady={false}>
          <div data-testid="deep-link-content" />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    const loadingState = screen.getByRole('status', { name: 'Loading shared query' });
    expect(loadingState.querySelector('svg')).toHaveClass('animate-spin');
    expect(screen.queryByTestId('deep-link-content')).not.toBeInTheDocument();

    rerender(
      <JotaiProvider>
        <DeepLinkBoundary durationSeconds={100} encodedState={encoded.value} isQueryReady>
          <div data-testid="deep-link-content" />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(screen.queryByRole('status', { name: 'Loading shared query' })).not.toBeInTheDocument();
    expect(screen.getByTestId('deep-link-content')).toBeInTheDocument();
  });

  it('does not subscribe to render-time timeline hydration', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    render(
      <JotaiProvider>
        <DeepLinkBoundary durationSeconds={100} isQueryReady>
          <HydrateTimelineDuringRender />
        </DeepLinkBoundary>
      </JotaiProvider>
    );

    expect(consoleError).not.toHaveBeenCalledWith(
      expect.stringContaining('Cannot update a component')
    );
  });

  it('shows an error toast for invalid incoming state without hydrating it', async () => {
    const toastSpy = vi.spyOn(toast, 'add');
    render(
      <>
        <JotaiProvider>
          <DeepLinkBoundary durationSeconds={100} encodedState="v1.invalid" isQueryReady>
            <IntakeStatusProbe />
            <ViewportProbe />
          </DeepLinkBoundary>
        </JotaiProvider>
        <Toaster />
      </>
    );

    expect(screen.getByTestId('intake-status')).toHaveTextContent('error');
    expect(screen.getByTestId('viewport')).toHaveTextContent(
      JSON.stringify({
        immediate: { start: 0, end: 0 },
        debounced: { start: 0, end: 0 },
      })
    );
    await waitFor(() =>
      expect(toastSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'deep-link-intake',
          type: 'error',
          title: 'Could not restore shared view',
        })
      )
    );
    await waitFor(() =>
      expect(document.querySelector('[data-slot="toast-title"]')).toHaveTextContent(
        'Could not restore shared view'
      )
    );
  });

  it('copies the current viewport without changing the address bar', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    window.history.replaceState(null, '', '/profile/engine/e/query/q/timeline?unrelated=kept');

    render(
      <>
        <div id={DEEP_LINK_NAV_SLOT_ID} />
        <JotaiProvider>
          <DeepLinkBoundary durationSeconds={100} isQueryReady>
            <SeedViewport start={20} end={60} />
            <SeedExpandedRows ids={[RESOURCE_B_ID, RESOURCE_A_ID]} />
            <ViewportProbe />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await waitFor(() => expect(screen.getByTestId('viewport')).toHaveTextContent('"start":20'));
    const originalUrl = window.location.href;
    await userEvent.click(screen.getByRole('button', { name: 'Copy Link' }));

    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());
    expect(screen.getByRole('button', { name: 'Copy Link' }).querySelector('svg')).toHaveClass(
      'lucide-check'
    );
    const copiedUrl = writeText.mock.calls[0][0] as string;
    const parsedUrl = new URL(copiedUrl);
    const encoded = parsedUrl.searchParams.get('s');
    expect(encoded).not.toBeNull();
    expect(parsedUrl.searchParams.has('unrelated')).toBe(false);
    expect(decodeDeepLinkState(encoded!)).toEqual({
      ok: true,
      value: {
        zoomRange: { start: 20, end: 60 },
        expandedResourceIds: [RESOURCE_A_ID, RESOURCE_B_ID],
      },
    });
    expect(window.location.href).toBe(originalUrl);
  });

  it('shows an error toast when copying fails', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error('denied')) },
    });
    const toastSpy = vi.spyOn(toast, 'add');

    render(
      <>
        <div id={DEEP_LINK_NAV_SLOT_ID} />
        <JotaiProvider>
          <DeepLinkBoundary durationSeconds={100} isQueryReady>
            <SeedViewport start={20} end={60} />
            <CopyLinkButton />
          </DeepLinkBoundary>
        </JotaiProvider>
      </>
    );

    await userEvent.click(await screen.findByRole('button', { name: 'Copy Link' }));
    await waitFor(() =>
      expect(toastSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'deep-link-copy-error',
          type: 'error',
          title: 'Could not copy link',
        })
      )
    );
  });
});
