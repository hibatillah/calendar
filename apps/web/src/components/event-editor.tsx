import { type EventInput } from "@calendar/api";
import { addDays, clockTime, dateKey, localInstant } from "@calendar/calendar";
import { useState } from "react";
import { Button } from "./ui/button";

export function EventEditor({
  draft,
  calendars,
  saving,
  onChange,
  onSave,
}: {
  draft: EventInput;
  calendars: { id: string; summary: string }[];
  saving: boolean;
  onChange: (event: EventInput) => void;
  onSave: () => void;
}) {
  const [error, setError] = useState("");
  const field = (patch: Partial<EventInput>) => onChange({ ...draft, ...patch });
  const updateInstant = (key: "start" | "end", value: string) => {
    try {
      const [date, time] = value.split("T");

      field({ [key]: localInstant(date, time) });
      setError("");
    } catch {
      setError("Choose a valid date and time.");
    }
  };

  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      <h2 className="font-semibold">{draft.readOnly ? "Event details" : "Event"}</h2>
      <fieldset disabled={saving || draft.readOnly} className="space-y-4">
        <input
          aria-label="Event title"
          placeholder="Add a title"
          value={draft.title}
          onChange={(event) => field({ title: event.target.value })}
          autoFocus
          required
        />
        <label>
          Calendar
          <select
            value={draft.calendarId}
            onChange={(event) => field({ calendarId: event.target.value })}
          >
            {calendars.map((calendar) => (
              <option key={calendar.id} value={calendar.id}>
                {calendar.summary}
              </option>
            ))}
          </select>
        </label>
        <label>
          Starts
          <input
            type="datetime-local"
            value={`${dateKey(new Date(draft.start))}T${clockTime(draft.start)}`}
            onChange={(event) => updateInstant("start", event.target.value)}
            required
          />
        </label>
        <label>
          Ends (exclusive for all-day)
          <input
            type="datetime-local"
            value={`${dateKey(new Date(draft.end))}T${clockTime(draft.end)}`}
            onChange={(event) => updateInstant("end", event.target.value)}
            required
          />
        </label>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            className="w-auto"
            checked={draft.allDay}
            onChange={(event) => {
              const allDay = event.target.checked;

              field({
                allDay,
                ...(allDay
                  ? {
                      start: localInstant(dateKey(new Date(draft.start)), "00:00"),
                      end: localInstant(
                        addDays(dateKey(new Date(draft.start)), 1),
                        "00:00",
                      ),
                    }
                  : {}),
              });
            }}
          />
          All-day
        </label>
        <label>
          Location
          <input
            value={draft.location}
            onChange={(event) => field({ location: event.target.value })}
          />
        </label>
        <label>
          Notes
          <textarea
            rows={2}
            value={draft.description}
            onChange={(event) => field({ description: event.target.value })}
          />
        </label>
      </fieldset>
      {error && <p role="alert">{error}</p>}
      <p className="text-xs text-muted-foreground">
        Times in Asia/Jakarta. Recurring edits affect this occurrence.
      </p>
      {!draft.readOnly && (
        <Button type="submit" disabled={saving || !!error} className="w-full">
          {saving ? "Saving…" : "Save event"}
        </Button>
      )}
    </form>
  );
}
