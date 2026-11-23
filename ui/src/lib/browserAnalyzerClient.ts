// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  createBrowserAnalyzerClient,
  createBrowserAnalyzerWorker,
  setApiClient,
} from '@quent/client';

export interface BrowserAnalyzerDefaults {
  modulePath: string;
  dataPath: string;
}

/** Install a worker-backed analyzer for the static Quent application. */
export async function installBrowserAnalyzerClient(
  defaults: BrowserAnalyzerDefaults
): Promise<void> {
  const search = new URLSearchParams(window.location.search);
  const configuredDataUrl = search.get('data');
  const configuredModuleUrl = search.get('module');
  const dataUrl =
    configuredDataUrl !== null
      ? new URL(configuredDataUrl, window.location.href).href
      : new URL(`${import.meta.env.BASE_URL}${defaults.dataPath}`, window.location.origin).href;
  const moduleUrl =
    configuredModuleUrl !== null
      ? new URL(configuredModuleUrl, window.location.href)
      : new URL(`${import.meta.env.BASE_URL}${defaults.modulePath}`, window.location.origin);
  if (moduleUrl.origin !== window.location.origin) {
    throw new Error('browser analyzer modules must be hosted on the Quent UI origin');
  }
  const worker = createBrowserAnalyzerWorker();
  setApiClient(
    await createBrowserAnalyzerClient(worker, {
      moduleUrl: moduleUrl.href,
      dataUrl,
    })
  );
}
