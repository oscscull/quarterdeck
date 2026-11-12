import type { Priority } from "./types";

/** The line and its context for a priority: the question, or what the user's turn needs. */
export function describe(p: Priority): { text: string; context: string } {
  if (p.kind === "ask") {
    return { text: p.ask.question, context: p.ask.context ? `${p.ask.context} · ${p.itemId}` : p.itemId };
  }
  if (p.kind === "yours" && p.need) return { text: p.need, context: `${p.itemTitle} · ${p.itemId}` };
  // An older peer may send no need, or kinds this build doesn't know: show the item.
  return { text: p.itemTitle, context: `${p.itemLabel} · ${p.itemId}` };
}

/** Stable across refreshes and machines. */
export const keyOf = (p: Priority): string =>
  `${p.node ?? ""}|${p.kind}|${p.kind === "ask" ? p.ask.id : p.itemId}`;
