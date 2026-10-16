import { test, expect } from '@playwright/test';
test.beforeEach(async ({ page }) => {
  await page.clock.install({ time: new Date('2026-10-15T03:00:00Z') });
  await page.addInitScript(() => sessionStorage.setItem('calendar.local.connection', JSON.stringify({ endpoint: 'http://127.0.0.1:47821/graphql', token: 'test-only-token' })));
  await page.route('http://127.0.0.1:47821/graphql', route => route.fulfill({ headers: { 'access-control-allow-origin': '*', 'access-control-allow-headers': '*' }, json: { data: { googleConnection: { configured: true, connected: true, connecting: false, message: '', accounts: [] }, events: [{ eventId: 'test', calendarId: 'test|calendar', title: 'Mobile fixture', start: '2026-10-15T02:00:00Z', end: '2026-10-15T03:00:00Z', timezone: 'Asia/Jakarta', allDay: false, description: '', location: '', color: 13399664, revision: 1, readOnly: true }] } } }));
  await page.goto('/');
});
test('selected day shows its ordered agenda and changing days updates it', async ({ page }) => {
  await expect(page.getByText('Mobile fixture', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '2026-10-15, 1 events', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '2026-10-16, 0 events', exact: true }).click();
  await expect(page.getByText('No events for this day.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Today', exact: true }).click();
  await expect(page.getByText('Mobile fixture', { exact: true })).toBeVisible();
});
test('settings and read-only event details can be opened and dismissed', async ({ page }) => {
  await page.getByRole('button', { name: 'Open Mobile fixture', exact: true }).click();
  await expect(page.getByLabel('Event title')).toHaveValue('Mobile fixture');
  await expect(page.getByLabel('Event title')).not.toBeEditable();
  await page.getByRole('button', { name: 'Close event', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByLabel('Backend URL')).toBeVisible();
  await page.getByRole('button', { name: 'Close settings', exact: true }).click();
});
