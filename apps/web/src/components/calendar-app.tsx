import { monthLabel } from "@calendar/calendar";
import {
  CalendarDays,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  PanelLeft,
  Settings,
  SquarePen,
  X,
} from "lucide-react";
import { CalendarSettings } from "./calendar-settings";
import { CalendarSidebar } from "./calendar-sidebar";
import { EventEditor } from "./event-editor";
import { MonthCalendar } from "./month-calendar";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { useCalendarController } from "./use-calendar-controller";
import { WeekCalendar } from "./week-calendar";

const weekdayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function CalendarApp() {
  const calendarState = useCalendarController();
  const {
    selectedDate,
    firstDay,
    weekStart,
    view,
    setView,
    sidebarOpen,
    setSidebarOpen,
    setSettingsOpen,
    draft,
    setDraft,
    message,
    setMessage,
    accountBusy,
    calendar,
    accounts,
    calendars,
    goToday,
    navigate,
    newEvent,
    saveEvent,
    disconnectAccount,
  } = calendarState;

  return (
    <div className="calendar-shell">
      {sidebarOpen && <CalendarSidebar state={calendarState} />}
      <main className="main-calendar">
        <header className="toolbar">
          <div className="flex items-center gap-3">
            {!sidebarOpen && (
              <Button
                variant="ghost"
                size="icon"
                aria-label="Open sidebar"
                onClick={() => setSidebarOpen(true)}
              >
                <PanelLeft size={18} />
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  aria-label="Account menu"
                  className="relative rounded-full w-8 h-8 bg-secondary overflow-hidden flex items-center justify-center"
                >
                  <CalendarDays size={18} />
                  {accounts[0]?.picture && (
                    <img
                      className="absolute inset-0 h-full w-full object-cover"
                      onError={(event) => {
                        event.currentTarget.style.display = "none";
                      }}
                      src={accounts[0].picture}
                      alt=""
                      referrerPolicy="no-referrer"
                    />
                  )}
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-64">
                <div className="px-2 py-3">
                  <strong>{accounts[0]?.name || "Calendar"}</strong>
                  <p className="text-xs text-muted-foreground truncate">
                    {accounts[0]?.email || "Local preview"}
                  </p>
                </div>
                <DropdownMenuItem onSelect={() => setSettingsOpen(true)}>
                  <Settings size={16} />
                  Settings
                </DropdownMenuItem>
                {accounts[0] && (
                  <DropdownMenuItem
                    disabled={accountBusy}
                    onSelect={() => void disconnectAccount(accounts[0].id)}
                  >
                    Disconnect this account
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div className="flex items-center gap-2">
            <Popover
              open={!!draft}
              onOpenChange={(open) => {
                if (!open && !calendar.save.isPending) setDraft(null);
              }}
            >
              <PopoverTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="New event"
                  onClick={() => newEvent()}
                >
                  <SquarePen size={17} />
                </Button>
              </PopoverTrigger>
              <PopoverContent
                align="end"
                className="w-[350px] max-h-[85vh] overflow-auto overscroll-contain"
              >
                {draft && (
                  <EventEditor
                    draft={draft}
                    calendars={calendars.filter((item) => item.writable)}
                    saving={calendar.save.isPending}
                    onChange={setDraft}
                    onSave={() =>
                      void saveEvent(
                        draft,
                        calendar.data?.events.some(
                          (event) => event.eventId === draft.eventId,
                        )
                          ? draft.revision
                          : -1,
                        true,
                      )
                    }
                  />
                )}
              </PopoverContent>
            </Popover>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="secondary">
                  {view === "week" ? "Week" : "Month"}
                  <ChevronDown size={14} />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent>
                <DropdownMenuItem onSelect={() => setView("week")}>
                  Week <span className="ml-auto text-muted-foreground">W</span>
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => setView("month")}>
                  Month <span className="ml-auto text-muted-foreground">M</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button variant="secondary" onClick={goToday}>
              Today
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Previous period"
              onClick={() => navigate(-1)}
            >
              <ChevronLeft size={19} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Next period"
              onClick={() => navigate(1)}
            >
              <ChevronRight size={19} />
            </Button>
          </div>
        </header>
        <h1 className="month-title">
          {monthLabel(view === "week" ? firstDay : selectedDate)}
        </h1>
        {calendar.isError && (
          <div role="alert" className="px-6 pb-3 text-amber-400 text-xs">
            {calendar.data ? "Offline snapshot · " : ""}
            {calendar.error.message}{" "}
            <button onClick={() => void calendar.refetch()} className="underline ml-2">
              Retry
            </button>
          </div>
        )}
        {calendar.isPending && (
          <p className="px-6 text-xs text-muted-foreground">Loading calendars…</p>
        )}
        {view === "month" ? (
          <>
            <div className="grid grid-cols-7 text-center text-muted-foreground pb-3">
              {Array.from({ length: 7 }, (_, index) => (
                <span key={index}>{weekdayNames[(index + weekStart) % 7]}</span>
              ))}
            </div>
            <MonthCalendar state={calendarState} />
          </>
        ) : (
          <WeekCalendar state={calendarState} />
        )}
      </main>
      <CalendarSettings state={calendarState} />
      {message && (
        <div role="status" className="status-toast flex gap-4 items-center">
          {message}
          {!calendar.save.isPending && (
            <button aria-label="Dismiss message" onClick={() => setMessage("")}>
              <X size={14} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
