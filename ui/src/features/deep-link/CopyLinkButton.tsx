// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Check, Share2 } from 'lucide-react';
import { Button, toast } from '@quent/components';
import { DEEP_LINK_NAV_SLOT_ID } from './deepLink.constants';
import { useDeepLink } from './deepLink.context';

type CopyFeedback =
  | { kind: 'idle'; message: '' }
  | { kind: 'working'; message: 'Copying…' }
  | { kind: 'success'; message: 'Link copied' }
  | { kind: 'error'; message: string };

export function CopyLinkButton() {
  const deepLink = useDeepLink();
  const [feedback, setFeedback] = useState<CopyFeedback>({ kind: 'idle', message: '' });
  const [portalTarget, setPortalTarget] = useState<HTMLElement | null>(null);
  const resetTimer = useRef<number | null>(null);

  useEffect(() => {
    setPortalTarget(document.getElementById(DEEP_LINK_NAV_SLOT_ID));
  }, []);

  useEffect(
    () => () => {
      if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
    },
    []
  );

  const handleCopy = async () => {
    if (!deepLink) return;
    setFeedback({ kind: 'working', message: 'Copying…' });
    const result = await deepLink.copyLink();
    setFeedback(
      result.ok
        ? { kind: 'success', message: 'Link copied' }
        : { kind: 'error', message: result.message }
    );
    if (!result.ok) {
      toast.add({
        id: 'deep-link-copy-error',
        type: 'error',
        title: 'Could not copy link',
        description: result.message,
        priority: 'high',
      });
    }

    if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
    resetTimer.current = window.setTimeout(() => setFeedback({ kind: 'idle', message: '' }), 3000);
  };

  const intakeMessage =
    deepLink?.intakeStatus.kind === 'warning' || deepLink?.intakeStatus.kind === 'error'
      ? deepLink.intakeStatus.message
      : null;
  const statusMessage = feedback.message || intakeMessage || '';
  const title = statusMessage || 'Copy Link';

  if (!portalTarget) return null;
  return createPortal(
    <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="h-9 w-9"
        aria-label="Copy Link"
        title={title}
        disabled={!deepLink || feedback.kind === 'working'}
        onClick={() => void handleCopy()}
      >
        {feedback.kind === 'success' ? <Check /> : <Share2 />}
      </Button>
      <span className="sr-only" aria-live="polite">
        {statusMessage}
      </span>
    </>,
    portalTarget
  );
}
