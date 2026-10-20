import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.route(
    "https://api.github.com/repos/hibatillah/calendar/releases?*",
    (route) => route.fulfill({ json: [] }),
  );
});

test("landing demo works without accessing a connected calendar", async ({ page }) => {
  const calendarRequests: string[] = [];

  page.on("request", (request) => {
    if (request.url().includes("/api/graphql")) calendarRequests.push(request.url());
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Your week,");
  await page.locator(".preview-event").first().click();
  await expect(page.getByText("EXAMPLE EVENT", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Month", exact: true }).click();
  await expect(page.getByLabel("Month preview", { exact: true })).toBeVisible();
  await expect(page.locator(".preview-large-month > div").last()).toHaveText("1");
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
  await expect(page.getByRole("navigation", { name: "Main navigation" })).toBeVisible();
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
  await expect(page.getByRole("link", { name: "Open web calendar" })).toHaveAttribute(
    "href",
    "/app",
  );
  await expect(page.locator(".platform-grid article")).toHaveCount(3);
  await page.goto("/privacy");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
});

test("downloads offer Windows MSI, Android and the web calendar", async ({ page }) => {
  const assetNames = [
    "calendar-windows-x64.msi",
    "calendar-android-preview.apk",
    "calendar-ios-simulator.tar.gz",
    "calendar-web-build.tar.gz",
    "SHA256SUMS.txt",
  ];
  const releaseUrl =
    "https://github.com/hibatillah/calendar/releases/download/preview-12";

  await page.route(
    "https://api.github.com/repos/hibatillah/calendar/releases?*",
    (route) =>
      route.fulfill({
        json: [
          {
            tag_name: "preview-12",
            draft: false,
            published_at: "2026-10-19T00:00:00Z",
            assets: assetNames.map((name) => ({
              name,
              size: 100,
              state: "uploaded",
              browser_download_url: `${releaseUrl}/${name}`,
            })),
          },
        ],
      }),
  );
  await page.goto("/download");
  await expect(
    page.getByRole("link", { name: "Download Windows MSI" }),
  ).toHaveAttribute("href", `${releaseUrl}/calendar-windows-x64.msi`);
  await expect(
    page.getByRole("link", { name: "Download Android APK" }),
  ).toHaveAttribute("href", `${releaseUrl}/calendar-android-preview.apk`);
  await expect(page.getByRole("link", { name: "Open web calendar" })).toHaveAttribute(
    "href",
    "/app",
  );
  await expect(page.getByRole("link", { name: /iOS simulator/ })).toHaveCount(0);
  await expect(page.getByRole("link", { name: /Download macOS/ })).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  for (const link of await page.locator(".platform-grid a").all()) {
    const fitsCard = await link.evaluate((element) => {
      const card = element.closest("article")!.getBoundingClientRect();
      const bounds = element.getBoundingClientRect();

      return bounds.left >= card.left && bounds.right <= card.right;
    });

    expect(fitsCard).toBe(true);
  }
});

test("downloads handle unavailable GitHub and missing release assets", async ({
  page,
}) => {
  await page.route(
    "https://api.github.com/repos/hibatillah/calendar/releases?*",
    (route) => route.fulfill({ status: 403, json: { message: "Rate limited" } }),
  );
  await page.goto("/download");
  await expect(page.getByRole("status")).toContainText("Could not load downloads");
  await expect(
    page.getByRole("link", { name: "Release notes and all files" }),
  ).toHaveAttribute("href", "https://github.com/hibatillah/calendar/releases");
  await expect(page.getByRole("link", { name: "Download Windows MSI" })).toHaveCount(0);
  await page.route(
    "https://api.github.com/repos/hibatillah/calendar/releases?*",
    (route) => route.fulfill({ json: [] }),
  );
  await page.reload();
  await expect(page.getByRole("status")).toContainText("first downloadable release");
});

test("navigation stays usable after scrolling and returns to its initial position", async ({
  page,
}) => {
  await page.goto("/");
  const navigation = page.locator(".marketing-nav");
  await expect(page.getByRole("button", { name: "Week", exact: true })).toBeEnabled();
  const initialBounds = await navigation.boundingBox();

  await page.evaluate(() => window.scrollTo(0, 1200));
  await expect(navigation).toHaveClass(/is-floating/);
  const floatingBounds = await navigation.boundingBox();

  expect(floatingBounds!.y).toBeGreaterThanOrEqual(0);
  expect(floatingBounds!.y).toBeLessThan(30);
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("link", { name: "Features" })
    .click();
  await expect(page).toHaveURL(/#features$/);
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(navigation).not.toHaveClass(/is-floating/);
  expect((await navigation.boundingBox())!.y).toBe(initialBounds!.y);
});
