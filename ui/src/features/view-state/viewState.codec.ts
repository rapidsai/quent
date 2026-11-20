// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { gunzipSync, gzipSync, strFromU8, strToU8 } from 'fflate';
import type { ZodError } from 'zod';
import {
  SUPPORTED_VIEW_STATE_SCHEMAS,
  type ViewStateVersion,
  type VersionedViewState,
} from './viewState.fields';
import { ViewStateV3Schema, MAX_ENCODED_STATE_LENGTH, type ViewStateV3 } from './viewState.schema';

export const CURRENT_VIEW_STATE_VERSION: ViewStateVersion = 'v3';
export const SUPPORTED_VIEW_STATE_VERSIONS = SUPPORTED_VIEW_STATE_SCHEMAS.map(
  entry => entry.version
);
export const VIEW_STATE_SEARCH_KEY = 's';
export const MAX_VIEW_STATE_URL_LENGTH = 2048;
export const MAX_DECOMPRESSED_STATE_LENGTH = 64 * 1024;

export interface ViewStateCodecOptions {
  maxEncodedLength?: number;
}

export type ViewStateErrorCode =
  | 'invalid-state'
  | 'invalid-encoding'
  | 'unsupported-version'
  | 'payload-too-large'
  | 'url-too-long'
  | 'invalid-url';

export type ViewStateResult<T> =
  | { ok: true; value: T }
  | {
      ok: false;
      code: ViewStateErrorCode;
      message: string;
      messages: readonly string[];
    };

function failure(
  code: ViewStateErrorCode,
  messageOrMessages: string | readonly string[]
): ViewStateResult<never> {
  const messages =
    typeof messageOrMessages === 'string' ? [messageOrMessages] : [...messageOrMessages];
  return { ok: false, code, message: messages.join(' '), messages };
}

function formatIssuePath(path: PropertyKey[]): string {
  return path.reduce<string>((result, segment) => {
    if (typeof segment === 'number') {
      return `${result}[${segment}]`;
    }
    const key = String(segment);
    return result ? `${result}.${key}` : key;
  }, '');
}

function invalidState(error: ZodError, fallback: string): ViewStateResult<never> {
  const messages = error.issues.map(issue => {
    const path =
      issue.code === 'too_big' || issue.code === 'custom' ? '' : formatIssuePath(issue.path);
    return path ? `${issue.message} (${path})` : issue.message;
  });
  return failure('invalid-state', messages.length > 0 ? messages : fallback);
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replace(/\+/gu, '-').replace(/\//gu, '_').replace(/=+$/u, '');
}

function base64UrlToBytes(value: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]+$/u.test(value)) {
    return null;
  }
  const padding = '='.repeat((4 - (value.length % 4)) % 4);
  try {
    const binary = atob(value.replace(/-/gu, '+').replace(/_/gu, '/') + padding);
    return Uint8Array.from(binary, char => char.charCodeAt(0));
  } catch {
    return null;
  }
}

export function encodeViewState(
  state: ViewStateV3,
  { maxEncodedLength = MAX_ENCODED_STATE_LENGTH }: ViewStateCodecOptions = {}
): ViewStateResult<string> {
  const parsed = ViewStateV3Schema.safeParse(state);
  if (!parsed.success) {
    return invalidState(parsed.error, 'The current shared view state is invalid.');
  }

  const json = JSON.stringify(parsed.data);
  const compressed = gzipSync(strToU8(json), { level: 9, mtime: 0 });
  const encoded = `${CURRENT_VIEW_STATE_VERSION}.${bytesToBase64Url(compressed)}`;

  if (encoded.length > maxEncodedLength) {
    return failure('payload-too-large', 'The encoded view state is too large.');
  }
  return { ok: true, value: encoded };
}

export function decodeViewState(
  encoded: string,
  { maxEncodedLength = MAX_ENCODED_STATE_LENGTH }: ViewStateCodecOptions = {}
): ViewStateResult<VersionedViewState> {
  if (encoded.length > maxEncodedLength) {
    return failure('payload-too-large', 'The view state exceeds the supported size.');
  }

  const separator = encoded.indexOf('.');
  if (separator === -1) {
    return failure('invalid-encoding', 'The view state has no version prefix.');
  }

  const version = encoded.slice(0, separator);
  const versionedSchema = SUPPORTED_VIEW_STATE_SCHEMAS.find(entry => entry.version === version);
  if (!versionedSchema) {
    return failure('unsupported-version', `Unsupported view-state version: ${version}`);
  }

  const compressed = base64UrlToBytes(encoded.slice(separator + 1));
  if (!compressed) {
    return failure('invalid-encoding', 'The view state is not valid URL-safe base64.');
  }

  try {
    const output = new Uint8Array(MAX_DECOMPRESSED_STATE_LENGTH + 1);
    const decompressed = gunzipSync(compressed, { out: output });
    if (decompressed.length > MAX_DECOMPRESSED_STATE_LENGTH) {
      return failure('payload-too-large', 'The decoded view state is too large.');
    }

    const json = JSON.parse(strFromU8(decompressed)) as unknown;
    const parsed = versionedSchema.schema.safeParse(json);
    if (!parsed.success) {
      return invalidState(parsed.error, `The view state does not match the ${version} schema.`);
    }
    return { ok: true, value: { version: versionedSchema.version, data: parsed.data } };
  } catch {
    return failure('invalid-encoding', 'The view state could not be decoded.');
  }
}

export function buildViewStateUrl(currentUrl: string, state: ViewStateV3): ViewStateResult<string> {
  const encoded = encodeViewState(state);
  if (!encoded.ok) {
    return encoded;
  }
  return buildViewStateUrlFromEncodedState(currentUrl, encoded.value);
}

export function buildViewStateUrlFromEncodedState(
  currentUrl: string,
  encodedState: string
): ViewStateResult<string> {
  const isAbsolute = /^[A-Za-z][A-Za-z\d+.-]*:/u.test(currentUrl);
  let url: URL;
  try {
    url = new URL(currentUrl, 'http://view-state.invalid');
  } catch {
    return failure('invalid-url', 'The current page URL is invalid.');
  }

  url.searchParams.set(VIEW_STATE_SEARCH_KEY, encodedState);
  const result = isAbsolute ? url.toString() : `${url.pathname}${url.search}${url.hash}`;
  if (result.length > MAX_VIEW_STATE_URL_LENGTH) {
    return failure('url-too-long', 'The shareable URL exceeds 2,048 characters.');
  }
  return { ok: true, value: result };
}
