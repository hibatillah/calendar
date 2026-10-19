import { test, expect } from "@playwright/test";

const snapshot = {
  googleConnection: {
    configured: true,
    connected: true,
    connecting: false,
    message: "",
    accounts: [
      {
        id: "test",
        name: "Calendar Tester",
        email: "tester@example.test",
        picture: "",
        eventAccess: true,
        writeAccess: true,
        connecting: false,
        message: "",
        calendars: [
          {
            id: "test|primary",
            summary: "Personal",
            primary: true,
            accessRole: "owner",
            backgroundColor: "#cc7670",
            timeZone: "Asia/Jakarta",
          },
        ],
      },
    ],
  },
  events: [],
};

test.beforeEach(async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-15T03:00:00Z") });
  await page.route("**/api/graphql", (route) =>
    route.fulfill({ json: { data: snapshot } }),
  );
  await page.goto("/app");
  await expect(page.getByText("tester@example.test", { exact: true })).toBeVisible();
});

test("Today, sidebar reopening, month shortcut and settings work", async ({ page }) => {
  await expect(page.getByText("tester@example.test", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Next period", exact: true }).click();
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.locator(".day-heading").first()).toHaveText("Mon 12");
  await expect(page.locator('.mini-grid button[aria-pressed="true"]')).toHaveCount(7);
  await page.getByRole("button", { name: "Close sidebar", exact: true }).click();
  await page.getByRole("button", { name: "Open sidebar", exact: true }).click();
  await page.keyboard.press("m");
  await expect(page.locator(".month-grid")).toBeVisible();
  await page.keyboard.press("w");
  await expect(page.locator(".week-scroller")).toBeVisible();
  await page.getByRole("button", { name: "Account menu" }).click();
  await page.getByRole("menuitem", { name: "Settings" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByLabel("Start week on").selectOption("0");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.locator(".day-heading").first()).toHaveText("Sun 11");
});

test("failed creates preserve the editor and entered title", async ({ page }) => {
  await page.getByRole("button", { name: "Create event", exact: true }).click();
  await page.getByRole("textbox", { name: "Event title" }).fill("Recoverable draft");
  await page.route("**/api/graphql", (route) =>
    route.fulfill({ json: { errors: [{ message: "Test provider unavailable" }] } }),
  );
  await page.getByRole("button", { name: "Save event", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Event title" })).toHaveValue(
    "Recoverable draft",
  );
  await expect(page.getByRole("status")).toContainText("Test provider unavailable");
});

test("local bridge rejects cross-origin requests", async ({ request }) => {
  const response = await request.post("/api/graphql", {
    headers: { Origin: "https://example.com" },
    data: { query: "{ mode }" },
  });

  expect(response.status()).toBe(403);
});

test("optimistic move stays at the target until a failed save rolls it back", async ({
  page,
}) => {
  const original = {
    eventId: "existing",
    calendarId: "test|primary",
    title: "Movable fixture",
    start: "2026-10-12T02:00:00Z",
    end: "2026-10-12T03:00:00Z",
    timezone: "Asia/Jakarta",
    allDay: false,
    description: "",
    location: "",
    color: 13399664,
    revision: 1,
    readOnly: false,
  };
  let releaseSave: () => void = () => {};
  const pendingSave = new Promise<void>((resolve) => {
    releaseSave = resolve;
  });

  await page.route("**/api/graphql", async (route) => {
    if (route.request().postDataJSON().query.includes("SaveCalendarEvent")) {
      await pendingSave;
      await route.fulfill({ json: { errors: [{ message: "Simulated conflict" }] } });
    } else await route.fulfill({ json: { data: { ...snapshot, events: [original] } } });
  });
  await page.reload();

  const originalColumn = page.locator(".day-column").nth(0);
  const targetColumn = page.locator(".day-column").nth(1);

  await expect(originalColumn.getByText("Movable fixture")).toBeVisible();
  await originalColumn.getByText("Movable fixture").click();
  await page.getByLabel("Starts", { exact: true }).fill("2026-10-13T09:00");
  await page
    .getByLabel("Ends (exclusive for all-day)", { exact: true })
    .fill("2026-10-13T10:00");
  await page.getByRole("button", { name: "Save event", exact: true }).click();

  try {
    await expect(targetColumn.getByText("Movable fixture")).toBeVisible();
    await expect(originalColumn.getByText("Movable fixture")).toHaveCount(0);
    await expect(page.getByRole("status")).toContainText("Saving");
  } finally {
    releaseSave();
  }

  await expect(originalColumn.getByText("Movable fixture")).toBeVisible();
  await expect(targetColumn.getByText("Movable fixture")).toHaveCount(0);
  await expect(page.getByRole("status")).toContainText("Simulated conflict");
});

test("appearance persists and System follows OS changes, including portals", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.getByRole("button", { name: "Account menu" }).click();
  await page.getByRole("menuitem", { name: "Settings" }).click();
  await page.getByLabel("Appearance", { exact: true }).selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByLabel("Appearance", { exact: true }).selectOption("light");
  await expect(page.getByRole("dialog")).toHaveCSS(
    "background-color",
    "rgb(255, 255, 255)",
  );
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByRole("button", { name: "Account menu" }).click();
  await page.getByRole("menuitem", { name: "Settings" }).click();
  await expect(page.getByLabel("Appearance", { exact: true })).toHaveValue("light");
  await page.getByLabel("Appearance", { exact: true }).selectOption("system");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
});
