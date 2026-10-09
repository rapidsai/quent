// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, type Locator, type Page, test } from '@playwright/test';

const ENGINE_ID = '01a07b4c-86ab-7971-97c1-24879c41910e';
const QUERY_ID = '01a07b4c-86ab-7971-97c1-28ffb10dde0d';
const QUERY_PATH = `/#/profile/engine/${ENGINE_ID}/query/${QUERY_ID}`;
const IS_RECORDING = process.env.PLAYWRIGHT_DEMO_RECORD === '1';

async function beat(page: Page, recordingMs = 800) {
  await page.waitForTimeout(IS_RECORDING ? recordingMs : 50);
}

async function installDemoCursor(page: Page) {
  await page.addInitScript(() => {
    const install = () => {
      if (document.querySelector('[data-testid="demo-cursor"]')) {
        return;
      }

      const style = document.createElement('style');
      style.textContent = `
        [data-testid="demo-cursor"] {
          position: fixed;
          left: 50%;
          top: 50%;
          width: 24px;
          height: 32px;
          pointer-events: none;
          z-index: 2147483647;
          filter: drop-shadow(0 1px 2px rgb(0 0 0 / 80%));
          transform: translate(-2px, -2px);
          transition: left 70ms ease-out, top 70ms ease-out;
        }
        .demo-click-pulse {
          position: fixed;
          width: 30px;
          height: 30px;
          pointer-events: none;
          z-index: 2147483646;
          border: 3px solid #facc15;
          border-radius: 9999px;
          animation: demo-click-pulse 550ms ease-out forwards;
        }
        @keyframes demo-click-pulse {
          from { opacity: 1; transform: translate(-50%, -50%) scale(0.35); }
          to { opacity: 0; transform: translate(-50%, -50%) scale(1.5); }
        }
      `;
      document.head.append(style);

      const cursor = document.createElement('div');
      cursor.dataset.testid = 'demo-cursor';
      cursor.dataset.clickCount = '0';
      cursor.innerHTML = `
        <svg viewBox="0 0 24 32" aria-hidden="true">
          <path d="M2 1v24l6.3-6.1 4.4 10.3 4.1-1.8-4.3-9.8H22L2 1Z"
            fill="white" stroke="#111827" stroke-width="1.6" stroke-linejoin="round" />
        </svg>
      `;
      document.body.append(cursor);

      window.addEventListener(
        'mousemove',
        event => {
          if (cursor.dataset.probing === 'true') {
            return;
          }
          cursor.style.left = `${event.clientX}px`;
          cursor.style.top = `${event.clientY}px`;
        },
        { capture: true }
      );
      window.addEventListener(
        'mousedown',
        event => {
          if (cursor.dataset.probing === 'true') {
            return;
          }
          cursor.dataset.clickCount = String(Number(cursor.dataset.clickCount) + 1);
          const pulse = document.createElement('div');
          pulse.className = 'demo-click-pulse';
          pulse.style.left = `${event.clientX}px`;
          pulse.style.top = `${event.clientY}px`;
          pulse.addEventListener('animationend', () => pulse.remove(), { once: true });
          document.body.append(pulse);
        },
        { capture: true }
      );
    };

    if (document.readyState === 'loading') {
      window.addEventListener('DOMContentLoaded', install, { once: true });
    } else {
      install();
    }
  });
}

async function humanClick(
  page: Page,
  target: Locator,
  options: { force?: boolean; settleMs?: number } = {}
) {
  if (!IS_RECORDING) {
    await target.click({ force: options.force });
    return;
  }

  await target.hover({ force: options.force });
  await page.waitForTimeout(160);
  await target.click({ force: options.force, delay: 90 });
  await page.waitForTimeout(options.settleMs ?? 100);
}

async function humanPointClick(page: Page, point: { x: number; y: number }) {
  await page.mouse.move(point.x, point.y, { steps: IS_RECORDING ? 12 : 2 });
  if (IS_RECORDING) {
    await page.waitForTimeout(140);
  }
  await page.mouse.down();
  if (IS_RECORDING) {
    await page.waitForTimeout(70);
  }
  await page.mouse.up();
  if (IS_RECORDING) {
    await page.waitForTimeout(100);
  }
}

async function drag(page: Page, from: { x: number; y: number }, to: { x: number; y: number }) {
  const steps = IS_RECORDING ? 24 : 4;
  await page.mouse.move(from.x, from.y, { steps: IS_RECORDING ? 12 : 2 });
  if (IS_RECORDING) {
    await page.waitForTimeout(120);
  }
  await page.mouse.down();
  if (IS_RECORDING) {
    await page.waitForTimeout(60);
  }
  for (let step = 1; step <= steps; step += 1) {
    const progress = step / steps;
    await page.mouse.move(from.x + (to.x - from.x) * progress, from.y + (to.y - from.y) * progress);
    if (IS_RECORDING) {
      await page.waitForTimeout(12);
    }
  }
  await page.mouse.up();
  if (IS_RECORDING) {
    await page.waitForTimeout(100);
  }
}

async function zoomDagAndPanUp(page: Page) {
  const pane = page.locator('.react-flow__pane');
  const box = await pane.boundingBox();
  expect(box).not.toBeNull();

  const focus = {
    x: box!.x + box!.width * 0.28,
    y: box!.y + box!.height * 0.72,
  };
  await page.mouse.move(focus.x, focus.y);
  await beat(page, 300);
  await page.mouse.wheel(0, -700);
  await beat(page);

  await drag(
    page,
    { x: box!.x + box!.width * 0.45, y: box!.y + box!.height * 0.7 },
    { x: box!.x + box!.width * 0.45, y: box!.y + box!.height * 0.35 }
  );
  await beat(page, 300);
  await page.mouse.wheel(0, -250);
  await beat(page, 900);
}

async function scrubPlayhead(page: Page) {
  const playhead = page.getByRole('slider', { name: 'Data flow playhead' });
  const track = page.getByTestId('dag-playhead');
  const playheadBox = await playhead.boundingBox();
  const trackBox = await track.boundingBox();
  expect(playheadBox).not.toBeNull();
  expect(trackBox).not.toBeNull();

  const y = playheadBox!.y + playheadBox!.height / 2;
  const x = playheadBox!.x + playheadBox!.width / 2;
  const distance = trackBox!.width * 0.22;
  await drag(page, { x, y }, { x: x + distance, y });
  await beat(page, 500);

  const movedBox = await playhead.boundingBox();
  expect(movedBox).not.toBeNull();
  await drag(
    page,
    { x: movedBox!.x + movedBox!.width / 2, y },
    { x: Math.max(trackBox!.x, movedBox!.x - distance * 0.75), y }
  );
}

async function zoomTimeline(page: Page, targetFraction: number) {
  const controller = page.getByTestId('timeline-controller');
  const box = await controller.boundingBox();
  expect(box).not.toBeNull();

  const initial = await controller.evaluate(element => ({
    start: Number(element.getAttribute('data-zoom-start')),
    end: Number(element.getAttribute('data-zoom-end')),
  }));
  const initialSpan = initial.end - initial.start;
  const targetSpan = initialSpan * targetFraction;

  await page.mouse.move(box!.x + box!.width * 0.52, box!.y + box!.height * 0.55);
  await beat(page, 250);
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const span = await controller.evaluate(
      element =>
        Number(element.getAttribute('data-zoom-end')) -
        Number(element.getAttribute('data-zoom-start'))
    );
    if (span <= targetSpan) {
      break;
    }
    await page.mouse.wheel(0, -420);
    await page.waitForTimeout(IS_RECORDING ? 90 : 20);
  }

  await expect
    .poll(async () =>
      controller.evaluate(
        element =>
          Number(element.getAttribute('data-zoom-end')) -
          Number(element.getAttribute('data-zoom-start'))
      )
    )
    .toBeLessThanOrEqual(targetSpan * 1.15);
}

async function openTimelineEntity(page: Page) {
  const gantts = page.locator('[data-long-entities-gantt]');
  await expect(gantts.first()).toBeVisible();
  const close = page.getByRole('button', { name: 'Close' });

  const tryOpen = async () => {
    for (let ganttIndex = 0; ganttIndex < (await gantts.count()); ganttIndex += 1) {
      const gantt = gantts.nth(ganttIndex);
      const box = await gantt.boundingBox();
      if (!box) {
        continue;
      }
      const probe = async (point: { x: number; y: number }) => {
        const cursor = page.getByTestId('demo-cursor');
        await cursor.evaluate(element => {
          element.dataset.probing = 'true';
        });
        await page.mouse.click(point.x, point.y);
        await cursor.evaluate(element => {
          element.dataset.probing = 'false';
        });
        if (!(await close.isVisible())) {
          return false;
        }

        await close.dispatchEvent('click');
        await expect(close).not.toBeVisible();
        await humanPointClick(page, point);
        await expect(close).toBeVisible();
        return true;
      };

      const shapes = gantt.locator('svg path, svg rect');
      for (let index = 0; index < Math.min(await shapes.count(), 80); index += 1) {
        const shapeBox = await shapes.nth(index).boundingBox();
        if (shapeBox && shapeBox.width > 3 && shapeBox.height > 3) {
          const point = {
            x: shapeBox.x + shapeBox.width / 2,
            y: shapeBox.y + shapeBox.height / 2,
          };
          if (await probe(point)) {
            return true;
          }
        }
      }
      for (const yFraction of [0.25, 0.45, 0.65]) {
        for (const xFraction of [0.42, 0.3, 0.55, 0.7, 0.18, 0.82]) {
          const point = {
            x: box.x + box.width * xFraction,
            y: box.y + Math.min(box.height - 4, box.height * yFraction),
          };
          if (await probe(point)) {
            return true;
          }
        }
      }
    }
    return false;
  };

  if (!(await tryOpen())) {
    await humanClick(page, page.getByRole('button', { name: 'Reset zoom' }));
    await beat(page);
    if (!(await tryOpen())) {
      throw new Error('No entity segment was clickable in the generated simulator run');
    }
  }
  return close;
}

async function selectOperatorType(page: Page, type: string) {
  const node = page
    .locator('.react-flow__node')
    .filter({ hasText: new RegExp(`\\d+:${type}`, 'i') })
    .first();
  await expect(node).toBeAttached();
  await humanClick(page, node, { force: true });
  await beat(page);
}

async function hoverStatHeaders(page: Page) {
  const headers = page.getByRole('columnheader').filter({ hasText: /duration|input|output/i });
  await expect(headers.first()).toBeVisible();

  const nodeSurface = page.locator('.react-flow__node > div > div').first();
  const initialColor = await nodeSurface.evaluate(
    element => getComputedStyle(element).backgroundColor
  );
  let observedHeatmap = false;
  for (let index = 0; index < Math.min(await headers.count(), 3); index += 1) {
    const header = headers.nth(index);
    await header.hover();
    await beat(page, 700);
    const color = await nodeSurface.evaluate(element => getComputedStyle(element).backgroundColor);
    observedHeatmap ||= color !== initialColor;
  }
  expect(observedHeatmap).toBe(true);
}

async function selectOnlyRequestedGroupFacets(page: Page, toolbar: Locator) {
  const facets = toolbar.getByRole('button');
  for (let index = 0; index < (await facets.count()); index += 1) {
    const facet = facets.nth(index);
    const name = (await facet.innerText()).replace(/\s+/g, ' ').trim();
    const shouldBePressed = /Worker \/ Plan/i.test(name) || /logical Operator Type/i.test(name);
    const isPressed = (await facet.getAttribute('aria-pressed')) === 'true';
    if (isPressed !== shouldBePressed) {
      await humanClick(page, facet);
    }
  }
}

test('exercises and records the README demo flow with generated simulator data', async ({
  page,
}) => {
  await installDemoCursor(page);
  await page.goto(`${QUERY_PATH}/timeline`);
  await expect(page.locator('.react-flow__node').first()).toBeVisible();
  await expect(page.getByRole('slider', { name: 'Data flow playhead' })).toBeVisible();
  const cursor = page.getByTestId('demo-cursor');
  await expect(cursor).toBeVisible();
  await beat(page, 1_200);

  await zoomDagAndPanUp(page);
  const play = page.getByRole('button', { name: 'Play data flow' });
  await humanClick(page, play);
  await expect(page.getByRole('button', { name: 'Pause data flow' })).toBeVisible();
  await expect
    .poll(async () => Number(await cursor.getAttribute('data-click-count')))
    .toBeGreaterThan(0);
  await beat(page, 1_800);

  const pane = page.locator('.react-flow__pane');
  const paneBox = await pane.boundingBox();
  expect(paneBox).not.toBeNull();
  await drag(
    page,
    { x: paneBox!.x + paneBox!.width * 0.5, y: paneBox!.y + paneBox!.height * 0.7 },
    { x: paneBox!.x + paneBox!.width * 0.5, y: paneBox!.y + paneBox!.height * 0.35 }
  );
  await beat(page, 1_200);

  await humanClick(page, page.getByRole('button', { name: 'Pause data flow' }));
  await expect(play).toBeVisible();
  await scrubPlayhead(page);
  await beat(page);
  const stop = page.getByRole('button', { name: 'Stop and clear playhead line' });
  await humanClick(page, stop);
  await expect(stop).toBeDisabled();
  await beat(page);

  await selectOperatorType(page, 'Join');
  await selectOperatorType(page, 'Project');
  const detailsToggle = page.getByRole('button', { name: 'Toggle operator details' });
  await expect(detailsToggle).toBeEnabled();
  await expect(page.getByRole('tab', { name: 'Stats' })).toBeVisible();

  const details = page.locator('[role="tabpanel"][data-state="active"]');
  await details.hover();
  await beat(page, 300);
  await page.mouse.wheel(0, 500);
  await beat(page, 1_200);

  await humanClick(page, page.getByRole('button', { name: 'Clear all filters' }));
  await expect(page.getByRole('button', { name: 'Clear all filters' })).toHaveCount(0);
  await beat(page);

  await humanClick(page, page.locator('[role="tree"] .chevron-icon[data-open="false"]').first());
  await expect(page.locator('[data-long-entities-gantt]').first()).toBeVisible();
  await zoomTimeline(page, 0.08);
  await beat(page, 1_000);
  await zoomTimeline(page, 0.25);
  await beat(page, 1_000);

  const closeEntity = await openTimelineEntity(page);
  await expect(page.getByText('Entity details', { exact: true })).toBeVisible();
  await beat(page);
  await humanClick(page, closeEntity);
  await expect(page.getByText('Entity details', { exact: true })).toHaveCount(0);

  await humanClick(page, page.getByRole('link', { name: 'Operators' }));
  const groupToolbar = page.getByText('Group by:', { exact: true }).locator('..');
  await expect(groupToolbar).toBeVisible();
  await selectOnlyRequestedGroupFacets(page, groupToolbar);
  await expect(groupToolbar.getByRole('button', { name: /Worker \/ Plan/i })).toHaveAttribute(
    'aria-pressed',
    'true'
  );
  await expect(
    groupToolbar.getByRole('button', { name: /logical\s+Operator Type/i })
  ).toHaveAttribute('aria-pressed', 'true');
  await expect(groupToolbar.getByRole('button', { pressed: true })).toHaveCount(2);
  await hoverStatHeaders(page);

  await humanClick(page, page.getByRole('link', { name: 'Entities' }));
  const minUsage = page.getByRole('spinbutton', { name: 'Min usage (s)' });
  await humanClick(page, minUsage);
  await minUsage.selectText();
  await minUsage.pressSequentially('0.021', { delay: IS_RECORDING ? 110 : 0 });
  await minUsage.press('Enter');
  await beat(page);
  const minUsageSlider = page
    .getByRole('group', { name: 'Min usage (s) slider' })
    .getByRole('slider');
  const maxUsage = Number(await minUsageSlider.getAttribute('aria-valuemax'));
  if (maxUsage < 0.021) {
    await minUsage.fill((maxUsage * 0.8).toFixed(4));
    await minUsage.press('Enter');
  }

  const operator = page.getByRole('combobox', { name: 'Operator' });
  await humanClick(page, operator);
  const joinOption = page.getByRole('option').filter({ hasText: /Join/i }).first();
  await expect(joinOption).toBeVisible();
  await humanClick(page, joinOption);
  await page.keyboard.press('Escape');

  const entityRow = page.locator('tbody tr').first();
  await expect(entityRow).toBeVisible();
  await humanClick(page, entityRow);
  await expect(page.getByRole('button', { name: 'Copy ID' })).toBeVisible();
  await beat(page, 1_500);
});
