import {
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useEffect } from "react";
import { addDays } from "@calendar/calendar";
import {
  CalendarSnapshotDocument,
  SaveCalendarEventDocument,
  type CalendarSnapshotQuery,
  type EventInput,
  type Transport,
} from "./index";
export interface SnapshotStorage {
  read(key: string): Promise<CalendarSnapshotQuery | undefined>;
  write(key: string, value: CalendarSnapshotQuery): Promise<void>;
  clear(): Promise<void>;
}
export function replaceOptimisticEvent(
  snapshot: CalendarSnapshotQuery,
  event: EventInput,
): CalendarSnapshotQuery {
  return {
    ...snapshot,
    events: [
      ...snapshot.events.filter(
        (previous) => previous.eventId !== event.eventId,
      ),
      event,
    ],
  };
}
export function useCalendar(
  transport: Transport,
  firstDay: string,
  storage: SnapshotStorage,
  enabled = true,
) {
  const queryClient = useQueryClient();
  const pendingWrites = useIsMutating({ mutationKey: ["calendar-save"] });
  const endDate = addDays(firstDay, 42);
  const queryKey = ["calendar", firstDay, endDate];
  const storageKey = `${firstDay}:${endDate}`;
  const query = useQuery({
    enabled: enabled && pendingWrites === 0,
    queryKey,
    queryFn: () =>
      transport(CalendarSnapshotDocument, { startDate: firstDay, endDate }),
    retry: false,
    refetchInterval: 30_000,
  });
  useEffect(() => {
    let active = true;
    if (!enabled) return;
    storage
      .read(storageKey)
      .then((snapshot) => {
        if (active && snapshot && !queryClient.getQueryData(queryKey))
          queryClient.setQueryData(queryKey, snapshot, { updatedAt: 1 });
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [storageKey, storage, queryClient, enabled]);
  useEffect(() => {
    if (enabled && query.data && !query.isError && pendingWrites === 0)
      storage.write(storageKey, query.data).catch(() => {});
  }, [query.data, query.isError, storageKey, storage, pendingWrites, enabled]);
  const save = useMutation({
    mutationKey: ["calendar-save"],
    mutationFn: (command: {
      event: EventInput;
      operationId: string;
      expectedRevision: number;
    }) => transport(SaveCalendarEventDocument, command),
    onMutate: async ({ event }) => {
      await queryClient.cancelQueries({ queryKey: ["calendar"] });
      const snapshots = queryClient.getQueriesData<CalendarSnapshotQuery>({
        queryKey: ["calendar"],
      });
      queryClient.setQueriesData<CalendarSnapshotQuery>(
        { queryKey: ["calendar"] },
        (previous) =>
          previous ? replaceOptimisticEvent(previous, event) : previous,
      );
      return { snapshots };
    },
    onError: (_error, _command, context) => {
      context?.snapshots.forEach(([key, snapshot]) =>
        queryClient.setQueryData(key, snapshot),
      );
    },
    onSuccess: (result, command) => {
      queryClient.setQueriesData<CalendarSnapshotQuery>(
        { queryKey: ["calendar"] },
        (previous) =>
          previous
            ? replaceOptimisticEvent(
                {
                  ...previous,
                  events: previous.events.filter(
                    (event) => event.eventId !== command.event.eventId,
                  ),
                },
                result.saveEvent,
              )
            : previous,
      );
    },
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["calendar"] }),
  });
  return { ...query, save };
}
