// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';
import { API_ENDPOINTS, waitForRequestsSettled } from './helpers';

test('loads the profile search page and lists profiles', async ({ page }) => {
  const queriesSettled = waitForRequestsSettled(page, API_ENDPOINTS.listQueries);
  await page.goto('/');

  await expect(page).toHaveTitle('Quent UI');
  await expect(page.getByRole('heading', { name: 'Search Profiles' })).toBeVisible();
  await expect(page.getByText('Search and filter query profiles')).toBeVisible();

  // Filter controls are present.
  await expect(page.getByLabel('Search profiles')).toBeVisible();
  await expect(page.getByLabel('Filter by engine')).toBeVisible();

  // The aggregated table surfaces the seeded query profile.
  await queriesSettled;
  const table = page.getByRole('table');
  await expect(table.getByText('test-query')).toBeVisible();

  // Selecting a row opens its profile view.
  await table.getByText('test-query').click();
  await expect(page).toHaveURL(/\/profile\/engine\/[^/]+\/query\/[^/]+/);
});
