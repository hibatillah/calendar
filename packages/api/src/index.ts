import { print } from "graphql";
import type { TypedDocumentNode } from "@graphql-typed-document-node/core";

export * from "./generated";

export type Transport = <Result, Variables extends Record<string, unknown>>(
  document: TypedDocumentNode<Result, Variables>,
  variables: Variables,
) => Promise<Result>;

export function createTransport(endpoint: string, token?: string): Transport {
  return async (document, variables) => {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(token ? { "x-calendar-token": token } : {}),
      },
      body: JSON.stringify({ query: print(document), variables }),
      signal: AbortSignal.timeout(30_000),
    });

    if (!response.ok)
      throw new Error(
        response.status === 401
          ? "Connection expired. Check your connection in Settings."
          : "Calendar service unavailable. Check the backend connection.",
      );

    const result = await response.json();

    if (result.errors?.length) throw new Error(result.errors[0].message);

    if (!result.data)
      throw new Error("The calendar service returned an empty response.");

    return result.data;
  };
}

export type { CalendarEventFieldsFragment as EventRecord } from "./generated";
