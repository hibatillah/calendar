import { startOfWeek, type WeekStart } from "@calendar/calendar";
import { CalendarDays, Plus, Settings } from "lucide-react";
import { storage } from "./snapshot-storage";
import { Button } from "./ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "./ui/dialog";
import type { CalendarController } from "./use-calendar-controller";
import { type CalendarTheme } from "./use-calendar-theme";

export function CalendarSettings({ state }: { state: CalendarController }) {
  const {
    theme,
    selectTheme,
    selectedDate,
    setFirstDay,
    weekStart,
    setWeekStart,
    settingsOpen,
    setSettingsOpen,
    settingsTab,
    setSettingsTab,
    accountBusy,
    calendar,
    accounts,
    log,
    connectAccount,
    disconnectAccount,
  } = state;

  return (
    <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
      <DialogContent className="sm:max-w-[1000px] h-[min(720px,85vh)] p-0 gap-0 overflow-hidden">
        <div className="flex min-h-0">
          <nav className="w-56 bg-secondary p-5 space-y-2">
            <p className="text-xs text-muted-foreground mb-5">Account</p>
            <Button
              variant={settingsTab === "general" ? "secondary" : "ghost"}
              className="w-full justify-start"
              onClick={() => setSettingsTab("general")}
            >
              <Settings size={16} />
              General
            </Button>
            <Button
              variant={settingsTab === "accounts" ? "secondary" : "ghost"}
              className="w-full justify-start"
              onClick={() => setSettingsTab("accounts")}
            >
              <CalendarDays size={16} />
              Google accounts
            </Button>
          </nav>
          <section className="flex-1 p-9 overflow-auto">
            <DialogTitle>
              {settingsTab === "general" ? "Calendar view" : "Calendar accounts"}
            </DialogTitle>
            <DialogDescription className="mt-3">
              {settingsTab === "general"
                ? "Choose how your calendar is displayed."
                : "Manage your Google accounts and calendar permissions."}
            </DialogDescription>
            {settingsTab === "general" ? (
              <div className="space-y-8 mt-8">
                <label className="max-w-60">
                  Appearance
                  <select
                    aria-label="Appearance"
                    value={theme}
                    onChange={(event) => {
                      selectTheme(event.target.value as CalendarTheme);
                      log("Appearance changed");
                    }}
                  >
                    <option value="system">System</option>
                    <option value="light">Light</option>
                    <option value="dark">Dark</option>
                  </select>
                </label>
                <label className="max-w-60">
                  Start week on
                  <select
                    value={weekStart}
                    onChange={(event) => {
                      const start = Number(event.target.value) as WeekStart;

                      setWeekStart(start);
                      localStorage.setItem("calendar:week-start", String(start));
                      setFirstDay(startOfWeek(selectedDate, start));
                    }}
                  >
                    <option value="1">Monday</option>
                    <option value="0">Sunday</option>
                  </select>
                </label>
                <p className="text-muted-foreground">Time zone: Asia/Jakarta (GMT+7)</p>
                <p className="text-muted-foreground">
                  Shortcuts: W — week · M — month · T — today
                </p>
                <Button
                  variant="secondary"
                  onClick={() => {
                    void storage.clear();
                    log("Offline snapshots cleared");
                  }}
                >
                  Clear offline snapshots
                </Button>
              </div>
            ) : (
              <div className="mt-8 space-y-5">
                {accounts.map((account) => (
                  <div key={account.id} className="rounded-lg border p-4">
                    <strong>{account.name}</strong>
                    <p className="text-muted-foreground text-xs mt-1">
                      {account.email}
                    </p>
                    <p className="text-xs text-muted-foreground mt-3">
                      {account.message}
                    </p>
                    <div className="flex gap-2 mt-3">
                      {(!account.eventAccess || !account.writeAccess) && (
                        <Button
                          variant="secondary"
                          disabled={accountBusy}
                          onClick={() => void connectAccount(account.id)}
                        >
                          Approve calendar access
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        disabled={accountBusy}
                        onClick={() => void disconnectAccount(account.id)}
                      >
                        Disconnect
                      </Button>
                    </div>
                  </div>
                ))}
                <Button
                  variant="secondary"
                  disabled={accountBusy}
                  onClick={() => void connectAccount()}
                >
                  <Plus size={16} />
                  Add Google account
                </Button>
                <p className="text-xs text-muted-foreground">
                  {calendar.data?.googleConnection.message}
                </p>
              </div>
            )}
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}
