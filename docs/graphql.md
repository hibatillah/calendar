# GraphQL guide

Run `./scripts/start.ps1`, then open http://127.0.0.1:47821/docs. Enter the value from `.local/session-token` and click **Connect**. Use the **Docs** panel to browse the schema and the play button to run an operation. The PostHog token stays in `.env`; never put it in this page.

GraphQL's schema is the API reference: types, fields, inputs and argument descriptions are exposed through introspection. In this repository, `///` comments and `#[graphql(desc = "...")]` argument annotations generate those descriptions. The same documentation is exported in `contracts/graphql/schema.graphql` and served publicly at `/schema.graphql`. Introspection through `/graphql` requires authentication. No calendar data or credentials appear in the public SDL.

## Read events

```graphql
query MyCalendar {
  mode
  events {
    eventId
    title
    start
    end
    timezone
    revision
  }
}
```

The mode is `LOCAL_PREVIEW`. Google synchronization is not implemented yet.

## Save an event

```graphql
mutation SaveEvent(
  $operationId: String!
  $expectedRevision: Int!
  $event: EventInput!
) {
  saveEvent(
    operationId: $operationId
    expectedRevision: $expectedRevision
    event: $event
  ) {
    eventId
    title
    revision
  }
}
```

Variables for a new sample event (use fresh UUIDs for your own operations/events):

```json
{
  "operationId": "7d1d2b91-7ea8-4d04-b91d-5796775500a1",
  "expectedRevision": -1,
  "event": {
    "eventId": "836a633b-fcfa-42da-aaeb-265b7bf5f4e8",
    "calendarId": "work",
    "title": "API sample meeting",
    "start": "2026-10-15T09:00:00+07:00",
    "end": "2026-10-15T10:00:00+07:00",
    "timezone": "Asia/Jakarta",
    "allDay": false,
    "description": "",
    "location": "",
    "color": 9215736,
    "revision": 0,
    "readOnly": false
  }
}
```

Use `expectedRevision: -1` to create. For an update, use the last returned revision and a new operation ID. Retry the exact same command using the same operation ID if a response is lost. The server ignores the input revision when assigning the next revision. All-day ends are exclusive; timestamps must include an offset.

## Errors and authentication

GraphQL execution errors are returned in the `errors` array, normally with HTTP 200. Check `errors` before using `data`. Application error codes appear in `errors[].extensions.code`:

| Code             | Meaning                                                                   |
| ---------------- | ------------------------------------------------------------------------- |
| `INVALID_EVENT`  | Invalid event values or timestamp                                         |
| `CONFLICT`       | Stale revision, missing event, or operation ID reused with different data |
| `READ_ONLY`      | Stored event cannot be edited                                             |
| `INTERNAL_ERROR` | Local persistence failed                                                  |

GraphQL syntax/schema validation errors also appear in `errors`, but may not have an application code. Missing/wrong authentication or an unapproved browser origin returns HTTP 401. Requests use the `x-calendar-token` header. The only allowed browser origin is `http://127.0.0.1:47821`; native callers omit Origin. GraphiQL assets are local, and the page never embeds or persists your session token.

For example, inspect schema documentation directly:

```graphql
query MutationDocumentation {
  __type(name: "Mutation") {
    fields {
      name
      description
      args {
        name
        description
      }
    }
  }
}
```

Sources: [GraphQL schema descriptions](https://graphql.org/learn/schema/), [introspection](https://graphql.org/learn/introspection/).

## Google connection

`query { googleConnection { configured connected connecting message calendars { id summary primary accessRole } } }` reports account setup and discovered calendars. `mutation { connectGoogle }` returns the URL to open in the system browser; `mutation { disconnectGoogle }` removes the local credential and attempts grant revocation. These operations use the same local session-token guard. Tokens are never returned through GraphQL. See [Google setup](google.md).
