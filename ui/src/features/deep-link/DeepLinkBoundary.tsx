// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useLayoutEffect, useMemo, useState, type ReactNode } from 'react';
import { useAtom } from 'jotai';
import { Loader2 } from 'lucide-react';
import { useReadZoomRange, useSetDebouncedZoomRange, useSetZoomRange } from '@quent/hooks';
import { toast } from '@quent/components';
import { expandedIdsAtom } from '@/atoms/resourceTree';
import { buildDeepLinkUrl, decodeDeepLinkState } from './deepLink.codec';
import {
  DeepLinkContext,
  type CopyLinkResult,
  type DeepLinkContextValue,
  type DeepLinkIntakeStatus,
} from './deepLink.context';
import { normalizeZoomRange, resolveCapturedZoomRange } from './deepLink.normalize';

interface DeepLinkBoundaryProps {
  children: ReactNode;
  durationSeconds: number;
  encodedState?: string;
  isQueryReady: boolean;
}

export function DeepLinkBoundary({
  children,
  durationSeconds,
  encodedState,
  isQueryReady,
}: DeepLinkBoundaryProps) {
  const readZoomRange = useReadZoomRange();
  const setZoomRange = useSetZoomRange();
  const setDebouncedZoomRange = useSetDebouncedZoomRange();
  const [expandedResourceIds, setExpandedResourceIds] = useAtom(expandedIdsAtom);

  const intake = useMemo(() => {
    if (!encodedState) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        isResolved: true,
        status: { kind: 'idle' } satisfies DeepLinkIntakeStatus,
      };
    }
    if (!isQueryReady) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        isResolved: false,
        status: { kind: 'idle' } satisfies DeepLinkIntakeStatus,
      };
    }

    const decoded = decodeDeepLinkState(encodedState);
    if (!decoded.ok) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        isResolved: true,
        status: { kind: 'error', message: decoded.message } satisfies DeepLinkIntakeStatus,
      };
    }

    const normalized = normalizeZoomRange(decoded.value.zoomRange, durationSeconds);
    if (!normalized) {
      return {
        initialExpandedResourceIds: decoded.value.expandedResourceIds ?? null,
        initialZoomRange: null,
        isResolved: true,
        status: {
          kind: 'error',
          message: 'The query duration cannot support the shared timeline viewport.',
        } satisfies DeepLinkIntakeStatus,
      };
    }

    return {
      initialExpandedResourceIds: decoded.value.expandedResourceIds ?? null,
      initialZoomRange: normalized.range,
      isResolved: true,
      status: normalized.wasAdjusted
        ? ({
            kind: 'warning',
            message: 'The shared timeline viewport was adjusted to fit this query.',
          } satisfies DeepLinkIntakeStatus)
        : ({ kind: 'ready' } satisfies DeepLinkIntakeStatus),
    };
  }, [durationSeconds, encodedState, isQueryReady]);

  const [isHydrated, setIsHydrated] = useState(!encodedState);

  useEffect(() => {
    if (intake.status.kind !== 'error' && intake.status.kind !== 'warning') return;

    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) return;
      if (intake.status.kind === 'error') {
        toast.add({
          id: 'deep-link-intake',
          type: 'error',
          title: 'Could not restore shared view',
          description: 'Some shared view settings were invalid and could not be restored.',
          priority: 'high',
        });
      } else if (intake.status.kind === 'warning') {
        toast.add({
          id: 'deep-link-intake',
          type: 'warning',
          title: 'Shared view adjusted',
          description: intake.status.message,
        });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [intake.status]);

  useLayoutEffect(() => {
    if (!intake.isResolved) return;
    if (intake.initialZoomRange) {
      setZoomRange(intake.initialZoomRange);
      setDebouncedZoomRange(intake.initialZoomRange);
    }
    if (intake.initialExpandedResourceIds) {
      setExpandedResourceIds(new Set(intake.initialExpandedResourceIds));
    }
    setIsHydrated(true);
  }, [
    intake.initialExpandedResourceIds,
    intake.initialZoomRange,
    intake.isResolved,
    setDebouncedZoomRange,
    setExpandedResourceIds,
    setZoomRange,
  ]);

  const copyLink = useCallback(async (): Promise<CopyLinkResult> => {
    const capturedRange = resolveCapturedZoomRange(readZoomRange(), durationSeconds);
    if (!capturedRange) {
      return { ok: false, message: 'The timeline viewport is not available yet.' };
    }

    const canonicalPageUrl = `${window.location.origin}${window.location.pathname}`;
    const result = buildDeepLinkUrl(canonicalPageUrl, {
      zoomRange: capturedRange,
      expandedResourceIds: [...expandedResourceIds].sort(),
    });
    if (!result.ok) return { ok: false, message: result.message };
    if (!navigator.clipboard?.writeText) {
      return { ok: false, message: 'Clipboard access is unavailable.' };
    }

    try {
      await navigator.clipboard.writeText(result.value);
      return { ok: true, url: result.value };
    } catch {
      return { ok: false, message: 'Could not copy the link to the clipboard.' };
    }
  }, [durationSeconds, expandedResourceIds, readZoomRange]);

  const value = useMemo<DeepLinkContextValue>(
    () => ({
      copyLink,
      initialExpandedResourceIds: intake.initialExpandedResourceIds,
      initialZoomRange: intake.initialZoomRange,
      intakeStatus: intake.status,
    }),
    [copyLink, intake.initialExpandedResourceIds, intake.initialZoomRange, intake.status]
  );

  return (
    <DeepLinkContext.Provider value={value}>
      {isHydrated ? (
        children
      ) : (
        <div
          role="status"
          aria-label="Loading shared query"
          className="flex min-h-[calc(100vh-4rem)] w-full items-center justify-center gap-2 text-muted-foreground"
        >
          <Loader2 className="size-5 animate-spin" aria-hidden="true" />
          <span>Loading shared view...</span>
        </div>
      )}
    </DeepLinkContext.Provider>
  );
}
