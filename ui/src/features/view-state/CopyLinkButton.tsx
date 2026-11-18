// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import { Check, Share2 } from 'lucide-react';
import { Button } from '@quent/components';
import { buildViewStateUrlFromEncodedState } from './viewState.codec';
import { useViewState } from './viewState.context';
import { showViewStateErrorToasts } from './viewState.toasts';

type CopyFeedback =
  | { kind: 'idle'; message: '' }
  | { kind: 'working'; message: 'Copying…' }
  | { kind: 'success'; message: 'Link copied' }
  | { kind: 'error'; message: string };

export function CopyLinkButton() {
  const viewState = useViewState();
  const [feedback, setFeedback] = useState<CopyFeedback>({ kind: 'idle', message: '' });
  const resetTimer = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current !== null) {
        window.clearTimeout(resetTimer.current);
      }
    },
    []
  );

  const handleCopy = async () => {
    if (!viewState) {
      return;
    }
    setFeedback({ kind: 'working', message: 'Copying…' });
    const captured = viewState.captureEncodedState();
    let result: { ok: true } | { ok: false; message: string; messages: readonly string[] } =
      captured;
    if (captured.ok) {
      const canonicalPageUrl = `${window.location.origin}${window.location.pathname}`;
      const link = buildViewStateUrlFromEncodedState(canonicalPageUrl, captured.value);
      if (!link.ok) {
        result = link;
      } else if (!navigator.clipboard?.writeText) {
        const message = 'Clipboard access is unavailable.';
        result = { ok: false, message, messages: [message] };
      } else {
        try {
          await navigator.clipboard.writeText(link.value);
          result = { ok: true };
        } catch {
          const message = 'Could not copy the link to the clipboard.';
          result = { ok: false, message, messages: [message] };
        }
      }
    }
    setFeedback(
      result.ok
        ? { kind: 'success', message: 'Link copied' }
        : { kind: 'error', message: result.message }
    );
    if (!result.ok) {
      showViewStateErrorToasts({
        id: 'view-state-copy-error',
        title: 'Could not copy link',
        messages: result.messages,
      });
    }

    if (resetTimer.current !== null) {
      window.clearTimeout(resetTimer.current);
    }
    resetTimer.current = window.setTimeout(() => setFeedback({ kind: 'idle', message: '' }), 3000);
  };

  const intakeMessage =
    viewState?.intakeStatus.kind === 'warning' || viewState?.intakeStatus.kind === 'error'
      ? viewState.intakeStatus.message
      : null;
  const statusMessage = feedback.message || intakeMessage || '';
  const title = statusMessage || 'Copy Link';

  return (
    <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-8"
        aria-label="Copy Link"
        title={title}
        disabled={!viewState || feedback.kind === 'working'}
        onClick={() => void handleCopy()}
      >
        {feedback.kind === 'success' ? <Check className="text-primary" /> : <Share2 />}
      </Button>
      <span className="sr-only" aria-live="polite">
        {statusMessage}
      </span>
    </>
  );
}
