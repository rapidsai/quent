// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

const DEFAULT_API_BASE_URL =
  (import.meta as ImportMeta & { env?: { VITE_API_BASE_URL?: string } }).env?.VITE_API_BASE_URL ??
  '/api';

let apiBaseUrl = DEFAULT_API_BASE_URL;

export function getApiBaseUrl(): string {
  return apiBaseUrl;
}

export function setApiBaseUrl(url: string): void {
  apiBaseUrl = url;
}

