// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, type Page } from '@playwright/test';

export interface BackendError {
  method: string;
  url: string;
  status?: number;
  failure?: string;
}

export interface UiError {
  source: 'page' | 'console' | 'dom';
  message: string;
  url?: string;
}

export interface CapturedErrors {
  ui: UiError[];
  backend: BackendError[];
}

export interface AllowedErrors {
  ui?: (error: UiError) => boolean;
  backend?: (error: BackendError) => boolean;
}

function isBackendRequest(url: string): boolean {
  const pathname = new URL(url).pathname;
  return pathname === '/api' || pathname.startsWith('/api/');
}

function formatBackendError(error: BackendError): string {
  if (error.status !== undefined) {
    return `${error.method} ${error.url} returned ${error.status}`;
  }
  return `${error.method} ${error.url} failed: ${error.failure ?? 'unknown error'}`;
}

export async function captureErrors(page: Page): Promise<CapturedErrors> {
  const errors: CapturedErrors = { ui: [], backend: [] };

  await page.exposeFunction('__quentCaptureDomError', (error: Omit<UiError, 'source'>) => {
    errors.ui.push({ source: 'dom', ...error });
  });
  await page.addInitScript(() => {
    const reported = new WeakMap<Element, string>();
    const captureDomErrors = () => {
      for (const heading of document.querySelectorAll('h2')) {
        if (heading.textContent?.trim() !== 'Something went wrong') {
          continue;
        }
        const message =
          heading.nextElementSibling?.textContent?.trim() || heading.textContent.trim();
        const fingerprint = `${window.location.href}\n${message}`;
        if (reported.get(heading) === fingerprint) {
          continue;
        }
        reported.set(heading, fingerprint);
        void (
          window as typeof window & {
            __quentCaptureDomError: (error: { message: string; url: string }) => Promise<void>;
          }
        ).__quentCaptureDomError({ message, url: window.location.href });
      }
    };

    new MutationObserver(captureDomErrors).observe(document, {
      childList: true,
      subtree: true,
      characterData: true,
    });
    window.addEventListener('DOMContentLoaded', captureDomErrors, { once: true });
  });

  page.on('pageerror', error =>
    errors.ui.push({ source: 'page', message: error.message, url: page.url() })
  );
  page.on('console', message => {
    if (message.type() === 'error') {
      errors.ui.push({
        source: 'console',
        message: message.text(),
        url: message.location().url || undefined,
      });
    }
  });
  page.on('response', response => {
    if (isBackendRequest(response.url()) && !response.ok()) {
      errors.backend.push({
        method: response.request().method(),
        url: response.url(),
        status: response.status(),
      });
    }
  });
  page.on('requestfailed', request => {
    if (isBackendRequest(request.url())) {
      errors.backend.push({
        method: request.method(),
        url: request.url(),
        failure: request.failure()?.errorText,
      });
    }
  });

  return errors;
}

export async function expectNoErrors(
  page: Page,
  errors: CapturedErrors,
  allowed: AllowedErrors = {}
) {
  const visibleDomErrors = await page
    .getByRole('heading', { name: 'Something went wrong' })
    .evaluateAll(headings =>
      headings.map(heading => ({
        source: 'dom' as const,
        message:
          heading.nextElementSibling?.textContent?.trim() || heading.textContent?.trim() || '',
        url: window.location.href,
      }))
    );
  for (const error of visibleDomErrors) {
    if (
      !errors.ui.some(
        captured =>
          captured.source === error.source &&
          captured.message === error.message &&
          captured.url === error.url
      )
    ) {
      errors.ui.push(error);
    }
  }

  expect(
    errors.ui.filter(error => !allowed.ui?.(error)),
    'Expected no unexpected UI errors'
  ).toEqual([]);
  expect(
    errors.backend.filter(error => !allowed.backend?.(error)).map(formatBackendError),
    'Expected no unexpected backend HTTP errors'
  ).toEqual([]);
}
