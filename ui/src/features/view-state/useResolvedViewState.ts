// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import type { ViewStateScope, EncodedViewStateSource } from './viewState.storage';

export type ResolvedViewState =
  | { status: 'idle'; encodedState: null; sourceId: null }
  | { status: 'loading'; encodedState: null; sourceId: null }
  | {
      status: 'ready';
      encodedState: string | null;
      sourceId: string | null;
      maxEncodedLength?: number;
    }
  | { status: 'error'; encodedState: null; sourceId: string; error: unknown };

interface UseResolvedViewStateOptions {
  enabled: boolean;
  resolutionKey: string | null;
  scope: ViewStateScope | null;
  sources: readonly EncodedViewStateSource[];
}

type Resolution = ResolvedViewState & { resolutionKey: string | null };

export function useResolvedViewState({
  enabled,
  resolutionKey,
  scope,
  sources,
}: UseResolvedViewStateOptions): ResolvedViewState {
  const activeKey = enabled && scope ? resolutionKey : null;
  const scopeRef = useRef(scope);
  const [resolution, setResolution] = useState<Resolution>({
    status: 'idle',
    encodedState: null,
    sourceId: null,
    resolutionKey: null,
  });

  useEffect(() => {
    scopeRef.current = scope;
  }, [scope]);

  useEffect(() => {
    const activeScope = scopeRef.current;
    if (!activeKey || !activeScope) {
      setResolution({
        status: 'idle',
        encodedState: null,
        sourceId: null,
        resolutionKey: null,
      });
      return;
    }
    let cancelled = false;
    setResolution({
      status: 'loading',
      encodedState: null,
      sourceId: null,
      resolutionKey: activeKey,
    });

    void (async () => {
      for (const source of sources) {
        try {
          const encodedState = await source.load(activeScope);
          if (encodedState !== null) {
            if (!cancelled) {
              setResolution({
                status: 'ready',
                encodedState,
                sourceId: source.id,
                maxEncodedLength: source.maxEncodedLength,
                resolutionKey: activeKey,
              });
            }
            return;
          }
        } catch (error) {
          if (!cancelled) {
            setResolution({
              status: 'error',
              encodedState: null,
              sourceId: source.id,
              error,
              resolutionKey: activeKey,
            });
          }
          return;
        }
      }
      if (!cancelled) {
        setResolution({
          status: 'ready',
          encodedState: null,
          sourceId: null,
          resolutionKey: activeKey,
        });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeKey, sources]);

  if (!activeKey) {
    return { status: 'idle', encodedState: null, sourceId: null };
  }
  if (resolution.resolutionKey !== activeKey) {
    return { status: 'loading', encodedState: null, sourceId: null };
  }
  const { resolutionKey: _resolutionKey, ...result } = resolution;
  return result;
}
