/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
/** A Google calendar event. Editing depends on account permissions and calendar access. */
export type EventInput = {
  /** All-day events use an exclusive end date. */
  allDay: boolean;
  /** Provider calendar identifier. */
  calendarId: string;
  /** Non-negative RGB integer, for example 9215736. */
  color: number;
  /** Optional notes, represented as an empty string when absent; maximum 16000 UTF-8 bytes. */
  description: string;
  /** Exclusive end instant in RFC 3339 format; must be after start. */
  end: string;
  /** Stable event identifier. Generate a UUID for a new event. */
  eventId: string;
  /** Location or meeting URL; empty when absent. */
  location: string;
  /** Whether edits are forbidden for this stored event. */
  readOnly: boolean;
  /** Server-managed revision. Use expectedRevision for concurrency checks. */
  revision: number;
  /** Inclusive start instant in RFC 3339 format, including a UTC offset. */
  start: string;
  /** IANA time zone used to display and edit local times, for example Asia/Jakarta. */
  timezone: string;
  /** Non-empty title, at most 1000 UTF-8 bytes. */
  title: string;
};

export type CalendarEventFieldsFragment = { eventId: string, calendarId: string, title: string, start: string, end: string, timezone: string, allDay: boolean, description: string, location: string, color: number, revision: number, readOnly: boolean };

export type CalendarSnapshotQueryVariables = Exact<{
  startDate: string;
  endDate: string;
}>;


export type CalendarSnapshotQuery = { googleConnection: { configured: boolean, connected: boolean, connecting: boolean, message: string, accounts: Array<{ id: string, name: string, email: string, picture: string, eventAccess: boolean, writeAccess: boolean, connecting: boolean, message: string, calendars: Array<{ id: string, summary: string, primary: boolean, accessRole: string, backgroundColor: string, timeZone: string }> }> }, events: Array<{ eventId: string, calendarId: string, title: string, start: string, end: string, timezone: string, allDay: boolean, description: string, location: string, color: number, revision: number, readOnly: boolean }> };

export type SaveCalendarEventMutationVariables = Exact<{
  operationId: string;
  expectedRevision: number;
  event: EventInput;
}>;


export type SaveCalendarEventMutation = { saveEvent: { eventId: string, calendarId: string, title: string, start: string, end: string, timezone: string, allDay: boolean, description: string, location: string, color: number, revision: number, readOnly: boolean } };

export type ConnectCalendarAccountMutationVariables = Exact<{
  accountId?: string | null | undefined;
}>;


export type ConnectCalendarAccountMutation = { connectGoogle: string };

export type DisconnectCalendarAccountMutationVariables = Exact<{
  accountId: string;
}>;


export type DisconnectCalendarAccountMutation = { disconnectGoogle: boolean };

export const CalendarEventFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CalendarEventFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EventRecord"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventId"}},{"kind":"Field","name":{"kind":"Name","value":"calendarId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"start"}},{"kind":"Field","name":{"kind":"Name","value":"end"}},{"kind":"Field","name":{"kind":"Name","value":"timezone"}},{"kind":"Field","name":{"kind":"Name","value":"allDay"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"location"}},{"kind":"Field","name":{"kind":"Name","value":"color"}},{"kind":"Field","name":{"kind":"Name","value":"revision"}},{"kind":"Field","name":{"kind":"Name","value":"readOnly"}}]}}]} as unknown as DocumentNode<CalendarEventFieldsFragment, unknown>;
export const CalendarSnapshotDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"CalendarSnapshot"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"startDate"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"endDate"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"googleConnection"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"configured"}},{"kind":"Field","name":{"kind":"Name","value":"connected"}},{"kind":"Field","name":{"kind":"Name","value":"connecting"}},{"kind":"Field","name":{"kind":"Name","value":"message"}},{"kind":"Field","name":{"kind":"Name","value":"accounts"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"email"}},{"kind":"Field","name":{"kind":"Name","value":"picture"}},{"kind":"Field","name":{"kind":"Name","value":"eventAccess"}},{"kind":"Field","name":{"kind":"Name","value":"writeAccess"}},{"kind":"Field","name":{"kind":"Name","value":"connecting"}},{"kind":"Field","name":{"kind":"Name","value":"message"}},{"kind":"Field","name":{"kind":"Name","value":"calendars"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"summary"}},{"kind":"Field","name":{"kind":"Name","value":"primary"}},{"kind":"Field","name":{"kind":"Name","value":"accessRole"}},{"kind":"Field","name":{"kind":"Name","value":"backgroundColor"}},{"kind":"Field","name":{"kind":"Name","value":"timeZone"}}]}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"events"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"startDate"},"value":{"kind":"Variable","name":{"kind":"Name","value":"startDate"}}},{"kind":"Argument","name":{"kind":"Name","value":"endDate"},"value":{"kind":"Variable","name":{"kind":"Name","value":"endDate"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CalendarEventFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CalendarEventFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EventRecord"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventId"}},{"kind":"Field","name":{"kind":"Name","value":"calendarId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"start"}},{"kind":"Field","name":{"kind":"Name","value":"end"}},{"kind":"Field","name":{"kind":"Name","value":"timezone"}},{"kind":"Field","name":{"kind":"Name","value":"allDay"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"location"}},{"kind":"Field","name":{"kind":"Name","value":"color"}},{"kind":"Field","name":{"kind":"Name","value":"revision"}},{"kind":"Field","name":{"kind":"Name","value":"readOnly"}}]}}]} as unknown as DocumentNode<CalendarSnapshotQuery, CalendarSnapshotQueryVariables>;
export const SaveCalendarEventDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SaveCalendarEvent"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"operationId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"expectedRevision"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"event"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"EventInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"saveEvent"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"operationId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"operationId"}}},{"kind":"Argument","name":{"kind":"Name","value":"expectedRevision"},"value":{"kind":"Variable","name":{"kind":"Name","value":"expectedRevision"}}},{"kind":"Argument","name":{"kind":"Name","value":"event"},"value":{"kind":"Variable","name":{"kind":"Name","value":"event"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CalendarEventFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CalendarEventFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EventRecord"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventId"}},{"kind":"Field","name":{"kind":"Name","value":"calendarId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"start"}},{"kind":"Field","name":{"kind":"Name","value":"end"}},{"kind":"Field","name":{"kind":"Name","value":"timezone"}},{"kind":"Field","name":{"kind":"Name","value":"allDay"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"location"}},{"kind":"Field","name":{"kind":"Name","value":"color"}},{"kind":"Field","name":{"kind":"Name","value":"revision"}},{"kind":"Field","name":{"kind":"Name","value":"readOnly"}}]}}]} as unknown as DocumentNode<SaveCalendarEventMutation, SaveCalendarEventMutationVariables>;
export const ConnectCalendarAccountDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ConnectCalendarAccount"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"accountId"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"connectGoogle"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"accountId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"accountId"}}}]}]}}]} as unknown as DocumentNode<ConnectCalendarAccountMutation, ConnectCalendarAccountMutationVariables>;
export const DisconnectCalendarAccountDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DisconnectCalendarAccount"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"accountId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"disconnectGoogle"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"accountId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"accountId"}}}]}]}}]} as unknown as DocumentNode<DisconnectCalendarAccountMutation, DisconnectCalendarAccountMutationVariables>;