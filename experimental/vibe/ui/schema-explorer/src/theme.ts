// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export type ThemePreference = 'system' | 'light' | 'dark';
export type ResolvedTheme = Exclude<ThemePreference, 'system'>;

const STORAGE_KEY = 'quent-schema-explorer-theme';
const SYSTEM_DARK_QUERY = '(prefers-color-scheme: dark)';

export function readThemePreference(): ThemePreference {
  try {
    const value = window.localStorage.getItem(STORAGE_KEY);
    return value === 'light' || value === 'dark' ? value : 'system';
  } catch {
    return 'system';
  }
}

export function saveThemePreference(preference: ThemePreference): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    // The selected theme still applies for the current page lifetime.
  }
}

export function resolveTheme(
  preference: ThemePreference,
  prefersDark = window.matchMedia(SYSTEM_DARK_QUERY).matches,
): ResolvedTheme {
  return preference === 'system'
    ? prefersDark
      ? 'dark'
      : 'light'
    : preference;
}

export function applyTheme(theme: ResolvedTheme): void {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
}

export function observeTheme(
  preference: ThemePreference,
): () => void {
  const systemTheme = window.matchMedia(SYSTEM_DARK_QUERY);
  const update = (): void => {
    applyTheme(resolveTheme(preference, systemTheme.matches));
  };

  update();
  if (preference !== 'system') return () => {};

  systemTheme.addEventListener('change', update);
  return () => systemTheme.removeEventListener('change', update);
}
