// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext, useState, useEffect, useMemo } from 'react';
import type { ReactNode } from 'react';

export const THEME_LIGHT = 'light';
export const THEME_DARK = 'dark';
export type Theme = typeof THEME_LIGHT | typeof THEME_DARK;

const THEME_STORAGE_KEY = 'theme';
const PREFERS_DARK_QUERY = '(prefers-color-scheme: dark)';

function isTheme(value: unknown): value is Theme {
  return value === THEME_DARK || value === THEME_LIGHT;
}

/** Default theme: localStorage first, then system preference. */
function getInitialTheme(): Theme {
  if (typeof window === 'undefined') return THEME_LIGHT;
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    if (isTheme(saved)) return saved;
  } catch {
    // Ignore storage access errors and fall back to system preference.
  }
  return window.matchMedia(PREFERS_DARK_QUERY).matches ? THEME_DARK : THEME_LIGHT;
}

/** Sync theme to document class and persist to localStorage as the user's default. */
function syncThemeToDocument(theme: Theme) {
  if (typeof document !== 'undefined') {
    document.documentElement.classList.toggle(THEME_DARK, theme === THEME_DARK);
  }
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Ignore storage write errors.
  }
}

type ThemeContextValue = {
  theme: Theme;
  setTheme: (theme: Theme) => void;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(getInitialTheme);

  useEffect(() => {
    syncThemeToDocument(theme);
  }, [theme]);

  const contextValue = useMemo(() => ({ theme, setTheme: setThemeState }), [theme]);

  return <ThemeContext.Provider value={contextValue}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (value == null) {
    throw new Error('useTheme must be used within ThemeProvider');
  }
  return value;
}
