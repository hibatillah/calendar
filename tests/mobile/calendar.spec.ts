import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.clock.install({ time: new Date("2026-10-15T03:00:00Z") });
  await page.addInitScript(() =>
    sessionStorage.setItem(
      "calendar.local.connection",
      JSON.stringify({
        endpoint: "http://127.0.0.1:47821/graphql",
        token: "test-only-token",
      }),
    ),
  );
  await page.route("http://127.0.0.1:47821/graphql", (route) =>
    route.fulfill({
      headers: {
        "access-control-allow-origin": "*",
        "access-control-allow-headers": "*",
      },
      json: {
        data: {
          googleConnection: {
            configured: true,
            connected: true,
            connecting: false,
            message: "",
            accounts: [],
          },
          events: [
            {
              eventId: "test",
              calendarId: "test|calendar",
              title: "Mobile fixture",
              start: "2026-10-15T02:00:00Z",
              end: "2026-10-15T03:00:00Z",
              timezone: "Asia/Jakarta",
              allDay: false,
              description: "",
              location: "",
              color: 13399664,
              revision: 1,
              readOnly: true,
            },
          ],
        },
      },
    }),
  );
  await page.goto("/");
});

test("selected day shows its ordered agenda and changing days updates it", async ({
  page,
}) => {
  await expect(page.getByText("Mobile fixture", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "2026-10-15, 1 events", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "2026-10-16, 0 events", exact: true }).click();
  await expect(
    page.getByText("No events for this day.", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.getByText("Mobile fixture", { exact: true })).toBeVisible();
});

test("settings and read-only event details can be opened and dismissed", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Open Mobile fixture", exact: true }).click();
  await expect(page.getByLabel("Event title")).toHaveValue("Mobile fixture");
  await expect(page.getByLabel("Event title")).not.toBeEditable();
  await page.getByRole("button", { name: "Close event", exact: true }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByLabel("Backend URL")).toBeVisible();
  await page.getByRole("button", { name: "Close settings", exact: true }).click();
});

test("week-start preference survives reopening the app", async ({ page }) => {
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByText("Sunday", { exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(localStorage.getItem("calendar.preferences") ?? "{}").weekStart,
      ),
    )
    .toBe(0);
  await page.reload();
  await expect(
    page.getByRole("button", { name: "2026-09-27, 0 events", exact: true }),
  ).toBeVisible();
});

test("failed save keeps edits and closing asks before discarding", async ({ page }) => {
  await page.route("http://127.0.0.1:47821/graphql", (route) => {
    const query = route.request().postDataJSON()?.query ?? "";

    return route.fulfill({
      headers: {
        "access-control-allow-origin": "*",
        "access-control-allow-headers": "*",
      },
      json: query.includes("mutation")
        ? { errors: [{ message: "Test save rejected" }] }
        : {
            data: {
              googleConnection: {
                configured: true,
                connected: true,
                connecting: false,
                message: "",
                accounts: [
                  {
                    id: "account",
                    name: "Test",
                    email: "test@example.invalid",
                    picture: "",
                    writeAccess: true,
                    calendars: [
                      {
                        id: "test|calendar",
                        summary: "Test calendar",
                        primary: true,
                        accessRole: "owner",
                        backgroundColor: "#888888",
                      },
                    ],
                  },
                ],
              },
              events: [],
            },
          },
    });
  });
  await page.reload();
  await page.getByRole("button", { name: "Create event", exact: true }).click();
  await page.getByText("Save event", { exact: true }).click();
  await expect(page.getByText("Add an event title.", { exact: true })).toBeVisible();
  await page.getByLabel("Event title").fill("Unsaved fixture");
  await page.getByText("Save event", { exact: true }).click();
  await expect(page.getByText("Test save rejected", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Event title")).toHaveValue("Unsaved fixture");
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.getByRole("button", { name: "Close event", exact: true }).click();
  await expect(page.getByLabel("Event title")).toHaveValue("Unsaved fixture");
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Close event", exact: true }).click();
  await expect(page.getByLabel("Event title")).not.toBeVisible();
});
