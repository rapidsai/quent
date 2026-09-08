// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { AllowedErrors, BackendError, UiError } from './error-capture';

function isMissingNvtxCatalog(error: BackendError): boolean {
  const pathname = new URL(error.url).pathname;
  return (
    error.method === 'GET' &&
    error.status === 404 &&
    /^\/api\/nvtx\/contexts\/[^/]+\/catalog$/.test(pathname)
  );
}

function isMissingNvtxCatalogUiError(error: UiError): boolean {
  if (!/404|not found/i.test(error.message)) {
    return false;
  }
  return [error.url, error.message].some(value => {
    if (!value) {
      return false;
    }
    return /\/api\/nvtx\/contexts\/[^/\s?]+\/catalog(?:[?\s]|$)/.test(value);
  });
}

export const allowedMissingNvtxCatalogErrors: AllowedErrors = {
  ui: isMissingNvtxCatalogUiError,
  backend: isMissingNvtxCatalog,
};
