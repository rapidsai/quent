// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, type Locator, type Page, test } from '@playwright/test';
import { allowedMissingNvtxCatalogErrors } from './utils/allowed-errors';
import { captureErrors, expectNoErrors } from './utils/error-capture';

const ENGINE_ID = '00000000-0000-0000-0000-000000000001';
const QUERY_ID = '00000000-0000-0000-0000-000000000004';
const QUERY_PATH = `/profile/engine/${ENGINE_ID}/query/${QUERY_ID}`;
const QUERY_DURATION_SECONDS = 7;

const LOGICAL_AGGREGATE_ID = '00000000-0000-0000-0000-000000000009';
const PARTIAL_AGGREGATE_W0_ID = '00000000-0000-0000-0000-00000000000d';
const FINAL_AGGREGATE_ID = '00000000-0000-0000-0000-00000000000e';
const PARTIAL_AGGREGATE_W1_ID = '00000000-0000-0000-0000-000000000030';

async function openTimeline(page: Page) {
  const errors = await captureErrors(page);
  const response = await page.goto(`${QUERY_PATH}/timeline`);

  expect(response?.ok()).toBe(true);
  await expect(page).toHaveURL(new RegExp(`${QUERY_PATH}/timeline$`));
  await expect(page.locator(`.react-flow__node[data-id="${LOGICAL_AGGREGATE_ID}"]`)).toBeVisible();
  return errors;
}

async function openOperatorGanttCharts(page: Page) {
  await page.getByText('worker-0', { exact: true }).click();
  await page.getByText('worker-1', { exact: true }).click();
  await expect(
    page.getByRole('group', { name: /Operator Gantt chart:.*FinalAggregate/ })
  ).toBeVisible();
  await expect(
    page.getByRole('group', { name: /Operator Gantt chart:.*PartialAggregate/ })
  ).toHaveCount(2);
}

async function clickGanttAtSecond(chart: Locator, second: number) {
  const box = await chart.boundingBox();
  expect(box).not.toBeNull();
  await chart.click({
    position: {
      x: ((box!.width - 10) * second) / QUERY_DURATION_SECONDS,
      y: box!.height / 2,
    },
  });
}

async function expectSelectedGanttOperators(page: Page, expectedIds: string[]) {
  await expect
    .poll(async () => {
      const selectedIds = await page
        .getByRole('group', { name: /^Operator Gantt chart:/ })
        .evaluateAll(charts =>
          charts.flatMap(chart =>
            (chart.getAttribute('data-selected-operator-ids') ?? '').split(' ').filter(Boolean)
          )
        );
      return selectedIds.sort();
    })
    .toEqual([...expectedIds].sort());
}

function dagNode(page: Page, operatorId: string) {
  return page.locator(`.react-flow__node[data-id="${operatorId}"]`);
}

function operatorOption(page: Page, label: string, location: string) {
  return page.getByRole('option').filter({ hasText: label }).filter({ hasText: location });
}

async function openEntityOperatorSelect(page: Page) {
  await page.getByRole('link', { name: 'Entities' }).click();
  const operatorSelect = page.getByRole('combobox', { name: 'Operator' });
  await expect(operatorSelect).toBeVisible();
  await operatorSelect.click();
}

test('logical operator selection stays synchronized across views and can be cleared', async ({
  page,
}) => {
  const errors = await openTimeline(page);
  await openOperatorGanttCharts(page);

  await dagNode(page, LOGICAL_AGGREGATE_ID).click();

  await expectSelectedGanttOperators(page, [
    PARTIAL_AGGREGATE_W0_ID,
    FINAL_AGGREGATE_ID,
    PARTIAL_AGGREGATE_W1_ID,
  ]);
  await expect(page.getByTestId('operator-details-title')).toContainText('Aggregate');
  await expect(page.getByTestId(`operator-accordion-${LOGICAL_AGGREGATE_ID}`)).toBeVisible();
  await expect(page.getByTestId(`operator-accordion-${PARTIAL_AGGREGATE_W0_ID}`)).toBeVisible();
  await expect(page.getByTestId(`operator-accordion-${FINAL_AGGREGATE_ID}`)).toBeVisible();
  await expect(page.getByTestId(`operator-accordion-${PARTIAL_AGGREGATE_W1_ID}`)).toBeVisible();

  await openEntityOperatorSelect(page);
  await expect(operatorOption(page, 'Aggregate', 'Plan: logical')).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await expect(operatorOption(page, 'PartialAggregate', 'Worker: worker-0')).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await expect(operatorOption(page, 'FinalAggregate', 'Worker: worker-0')).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await expect(operatorOption(page, 'PartialAggregate', 'Worker: worker-1')).toHaveAttribute(
    'aria-selected',
    'true'
  );

  await page.getByRole('link', { name: 'Timeline' }).click();
  const worker0Chart = page.getByRole('group', {
    name: /Operator Gantt chart:.*FinalAggregate/,
  });
  await clickGanttAtSecond(worker0Chart, 3.5);

  await expect(page.getByRole('button', { name: 'Remove Aggregate' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Remove FinalAggregate' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove PartialAggregate' })).toBeVisible();
  await expectSelectedGanttOperators(page, [FINAL_AGGREGATE_ID, PARTIAL_AGGREGATE_W1_ID]);
  await expect(page.getByTestId(`operator-accordion-${LOGICAL_AGGREGATE_ID}`)).toHaveCount(0);
  await expect(page.getByTestId('operator-details-title')).toContainText('FinalAggregate');
  await expect(page.getByTestId('operator-details-title')).toContainText('PartialAggregate');

  await page.getByRole('button', { name: 'Clear all operator filters' }).click();
  await expect(page.getByRole('button', { name: 'Clear all operator filters' })).toHaveCount(0);
  await expect(page.getByTestId('operator-details-title')).toHaveCount(0);
  await expectSelectedGanttOperators(page, []);

  await openEntityOperatorSelect(page);
  await expect(page.getByRole('combobox', { name: 'Operator' })).toHaveText('All operators');
  await expect(page.getByRole('option', { selected: true })).toHaveCount(0);
  await page.waitForLoadState('networkidle');
  await expectNoErrors(page, errors, allowedMissingNvtxCatalogErrors);
});

test('Gantt and entity operator selections stay synchronized with the DAG and details', async ({
  page,
}) => {
  const errors = await openTimeline(page);
  await openOperatorGanttCharts(page);
  await expect(page.getByRole('button', { name: 'Clear all operator filters' })).toHaveCount(0);

  const worker0Chart = page.getByRole('group', {
    name: /Operator Gantt chart:.*FinalAggregate/,
  });
  await clickGanttAtSecond(worker0Chart, 3.5);
  await clickGanttAtSecond(worker0Chart, 4.5);

  await expectSelectedGanttOperators(page, [PARTIAL_AGGREGATE_W0_ID, FINAL_AGGREGATE_ID]);
  await expect(dagNode(page, PARTIAL_AGGREGATE_W0_ID).locator('.border-2')).toBeVisible();
  await expect(dagNode(page, FINAL_AGGREGATE_ID).locator('.border-2')).toBeVisible();
  await expect(page.getByTestId('operator-details-title')).toContainText('PartialAggregate');
  await expect(page.getByTestId('operator-details-title')).toContainText('FinalAggregate');

  await openEntityOperatorSelect(page);
  const partialAggregate = operatorOption(page, 'PartialAggregate', 'Worker: worker-0');
  const finalAggregate = operatorOption(page, 'FinalAggregate', 'Worker: worker-0');
  await expect(partialAggregate).toHaveAttribute('aria-selected', 'true');
  await expect(finalAggregate).toHaveAttribute('aria-selected', 'true');

  await partialAggregate.click();
  await expect(partialAggregate).toHaveAttribute('aria-selected', 'false');
  await expect(dagNode(page, PARTIAL_AGGREGATE_W0_ID).locator('.border-2')).toHaveCount(0);
  await expect(dagNode(page, FINAL_AGGREGATE_ID).locator('.border-2')).toBeVisible();
  await expect(page.getByTestId('operator-details-title')).not.toContainText('PartialAggregate');
  await expect(page.getByTestId('operator-details-title')).toContainText('FinalAggregate');

  await finalAggregate.click();
  await expect(finalAggregate).toHaveAttribute('aria-selected', 'false');
  await expect(dagNode(page, FINAL_AGGREGATE_ID).locator('.border-2')).toHaveCount(0);
  await expect(page.getByTestId('operator-details-title')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Clear all operator filters' })).toHaveCount(0);

  await page.getByRole('link', { name: 'Timeline' }).click();
  await expectSelectedGanttOperators(page, []);
  await page.waitForLoadState('networkidle');
  await expectNoErrors(page, errors, allowedMissingNvtxCatalogErrors);
});
