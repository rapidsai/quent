// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';

test('loads the static WASM analyzer and navigates to a query', async ({ page }) => {
  const response = await page.goto('/');
  expect(response?.ok()).toBe(true);
  await expect(page.getByRole('heading', { name: 'Select an engine' })).toBeVisible();

  await page
    .getByRole('link', { name: /View queries:/ })
    .first()
    .click();
  await expect(page.getByRole('heading', { name: 'Select a query' })).toBeVisible();

  await page
    .getByRole('link', { name: /View profile:/ })
    .first()
    .click();
  await expect(page).toHaveURL(/\/timeline$/);
  await expect(page.getByRole('tree')).toBeVisible();
});
