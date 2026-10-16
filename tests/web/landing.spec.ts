import { test, expect } from "@playwright/test";

test("landing demo works without accessing a connected calendar", async ({
  page,
}) => {
  const calendarRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/graphql"))
      calendarRequests.push(request.url());
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(
    "Your week,",
  );
  await page.locator(".preview-event").first().click();
  await expect(page.getByText("EXAMPLE EVENT", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Month", exact: true }).click();
  await expect(page.getByLabel("Month preview", { exact: true })).toBeVisible();
  await expect(page.locator(".preview-large-month > div").last()).toHaveText(
    "1",
  );
  await page.getByRole("button", { name: "Week", exact: true }).click();
  await expect(page.locator(".preview-event").first()).toBeVisible();
  expect(calendarRequests).toEqual([]);
  await expect(
    page.getByRole("link", { name: "Get the Windows preview" }),
  ).toHaveAttribute("href", "/download");
});

test("mobile navigation, availability and privacy pages are usable", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.getByRole("button", { name: "Open navigation" }).click();
  await expect(
    page.getByRole("navigation", { name: "Main navigation" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Close navigation" }).click();
  await page.getByText("Is this ready for everyone?", { exact: true }).click();
  await expect(
    page.getByText("Not yet. This is a developer preview", { exact: false }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.goto("/download");
  await expect(
    page.getByText("TestFlight", { exact: false }).first(),
  ).toBeVisible();
  await page.goto("/privacy");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
});
