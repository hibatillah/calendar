import { ChevronDown, CircleHelp, PanelLeft, SquarePen } from "lucide-react";
import { MiniCalendar } from "./mini-calendar";
import { Button } from "./ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import type { CalendarController } from "./use-calendar-controller";

export function CalendarSidebar({ state }: { state: CalendarController }) {
  const {
    setSidebarOpen,
    setSettingsOpen,
    setSettingsTab,
    activityOpen,
    setActivityOpen,
    activity,
    hiddenCalendars,
    setHiddenCalendars,
    accounts,
    newEvent,
  } = state;

  return (
    <aside className="sidebar">
      <div className="flex items-center justify-between">
        <div className="flex gap-1">
          <ChevronDown size={18} className="mt-2 text-muted-foreground" />
          <Button
            variant="ghost"
            size="icon"
            aria-label="Close sidebar"
            onClick={() => setSidebarOpen(false)}
          >
            <PanelLeft size={18} />
          </Button>
        </div>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Create event"
          onClick={() => newEvent()}
        >
          <SquarePen size={18} />
        </Button>
      </div>
      <div>
        <MiniCalendar state={state} />
      </div>
      <div className="flex-1 overflow-auto space-y-5">
        {accounts.map((account) => (
          <section key={account.id}>
            <p
              className="text-xs text-muted-foreground mb-2 truncate"
              title={account.email}
            >
              {account.email}
            </p>
            {account.calendars.map((item) => (
              <button
                key={item.id}
                className="flex w-full items-center gap-3 rounded-md px-2 py-2 text-left hover:bg-secondary/30"
                onClick={() =>
                  setHiddenCalendars((previous) => {
                    const next = new Set(previous);

                    if (next.has(item.id)) {
                      next.delete(item.id);
                    } else {
                      next.add(item.id);
                    }

                    return next;
                  })
                }
                aria-pressed={!hiddenCalendars.has(item.id)}
              >
                <span
                  className="h-3.5 w-3.5 shrink-0 rounded"
                  style={{
                    background: hiddenCalendars.has(item.id)
                      ? "transparent"
                      : item.backgroundColor,
                    border: `1px solid ${item.backgroundColor}`,
                  }}
                />
                <span className="truncate">{item.summary}</span>
              </button>
            ))}
          </section>
        ))}
        {!accounts.length && (
          <div className="text-muted-foreground leading-6 px-2">
            Your calendars will appear here.
            <Button
              variant="ghost"
              onClick={() => {
                setSettingsTab("accounts");
                setSettingsOpen(true);
              }}
            >
              Connect Google
            </Button>
          </div>
        )}
      </div>
      <Popover open={activityOpen} onOpenChange={setActivityOpen}>
        <PopoverTrigger asChild>
          <Button variant="ghost" className="justify-start text-muted-foreground">
            <CircleHelp size={17} />
            Activity
          </Button>
        </PopoverTrigger>
        <PopoverContent
          className="w-80 max-h-72 overflow-auto overscroll-contain"
          side="right"
        >
          <h3 className="font-semibold mb-3">Activity</h3>
          {activity.length ? (
            activity.map((entry, index) => (
              <p key={index} className="text-xs text-muted-foreground py-2 border-b">
                {entry}
              </p>
            ))
          ) : (
            <p className="text-muted-foreground text-xs">No activity this session.</p>
          )}
        </PopoverContent>
      </Popover>
    </aside>
  );
}
