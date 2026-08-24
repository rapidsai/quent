// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { gunzipSync, gzipSync, strFromU8, strToU8 } from 'fflate';
import {
  DeepLinkStateV1Schema,
  MAX_ENCODED_STATE_LENGTH,
  type DeepLinkStateV1,
} from './deepLink.schema';

export const DEEP_LINK_VERSION = 'v1';
export const DEEP_LINK_SEARCH_KEY = 's';
export const MAX_DEEP_LINK_URL_LENGTH = 2048;
export const MAX_DECOMPRESSED_STATE_LENGTH = 64 * 1024;

export type DeepLinkErrorCode =
  | 'invalid-state'
  | 'invalid-encoding'
  | 'unsupported-version'
  | 'payload-too-large'
  | 'url-too-long'
  | 'invalid-url';

export type DeepLinkResult<T> =
  { ok: true; value: T } | { ok: false; code: DeepLinkErrorCode; message: string };

function failure(code: DeepLinkErrorCode, message: string): DeepLinkResult<never> {
  return { ok: false, code, message };
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/gu, '-').replace(/\//gu, '_').replace(/=+$/u, '');
}

function base64UrlToBytes(value: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]+$/u.test(value)) return null;
  const padding = '='.repeat((4 - (value.length % 4)) % 4);
  try {
    const binary = atob(value.replace(/-/gu, '+').replace(/_/gu, '/') + padding);
    return Uint8Array.from(binary, char => char.charCodeAt(0));
  } catch {
    return null;
  }
}

export function encodeDeepLinkState(state: DeepLinkStateV1): DeepLinkResult<string> {
  const parsed = DeepLinkStateV1Schema.safeParse(state);
  if (!parsed.success) {
    return failure('invalid-state', 'The current timeline viewport is invalid.');
  }

  const json = JSON.stringify(parsed.data);
  const compressed = gzipSync(strToU8(json), { level: 9, mtime: 0 });
  const encoded = `${DEEP_LINK_VERSION}.${bytesToBase64Url(compressed)}`;

  if (encoded.length > MAX_ENCODED_STATE_LENGTH) {
    return failure('payload-too-large', 'The encoded deep-link state is too large.');
  }
  return { ok: true, value: encoded };
}

export function decodeDeepLinkState(encoded: string): DeepLinkResult<DeepLinkStateV1> {
  if (encoded.length > MAX_ENCODED_STATE_LENGTH) {
    return failure('payload-too-large', 'The deep-link state exceeds the supported size.');
  }

  const separator = encoded.indexOf('.');
  if (separator === -1) {
    return failure('invalid-encoding', 'The deep-link state has no version prefix.');
  }

  const version = encoded.slice(0, separator);
  if (version !== DEEP_LINK_VERSION) {
    return failure('unsupported-version', `Unsupported deep-link version: ${version}`);
  }

  const compressed = base64UrlToBytes(encoded.slice(separator + 1));
  if (!compressed) {
    return failure('invalid-encoding', 'The deep-link state is not valid URL-safe base64.');
  }

  try {
    const output = new Uint8Array(MAX_DECOMPRESSED_STATE_LENGTH + 1);
    const decompressed = gunzipSync(compressed, { out: output });
    if (decompressed.length > MAX_DECOMPRESSED_STATE_LENGTH) {
      return failure('payload-too-large', 'The decoded deep-link state is too large.');
    }

    const json = JSON.parse(strFromU8(decompressed)) as unknown;
    const parsed = DeepLinkStateV1Schema.safeParse(json);
    if (!parsed.success) {
      return failure('invalid-state', 'The deep-link state does not match the v1 schema.');
    }
    return { ok: true, value: parsed.data };
  } catch {
    return failure('invalid-encoding', 'The deep-link state could not be decoded.');
  }
}

export function buildDeepLinkUrl(
  currentUrl: string,
  state: DeepLinkStateV1
): DeepLinkResult<string> {
  const encoded = encodeDeepLinkState(state);
  if (!encoded.ok) return encoded;

  const isAbsolute = /^[A-Za-z][A-Za-z\d+.-]*:/u.test(currentUrl);
  let url: URL;
  try {
    url = new URL(currentUrl, 'http://deep-link.invalid');
  } catch {
    return failure('invalid-url', 'The current page URL is invalid.');
  }

  url.searchParams.set(DEEP_LINK_SEARCH_KEY, encoded.value);
  const result = isAbsolute ? url.toString() : `${url.pathname}${url.search}${url.hash}`;
  if (result.length > MAX_DEEP_LINK_URL_LENGTH) {
    return failure('url-too-long', 'The shareable URL exceeds 2,048 characters.');
  }
  return { ok: true, value: result };
}
