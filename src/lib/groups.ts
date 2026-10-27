import type { CopyKey } from "./copy";
import type { Stage, Turn, WorkItem } from "./types";

export const STAGES: readonly Stage[] = ["up_next", "in_progress", "in_review", "done"];

export const isInFlight = (item: WorkItem): boolean =>
  item.stage !== "up_next" && item.stage !== "done";

/** Priorities can show everything in play, or only the work that is one side's turn. */
export type TurnFilter = "all" | Turn;

export const TURN_FILTERS: readonly { id: TurnFilter; label: CopyKey }[] = [
  { id: "all", label: "priorities.filter.all" },
  { id: "you", label: "priorities.filter.you" },
  { id: "agent", label: "priorities.filter.agent" },
  { id: "others", label: "priorities.filter.others" },
  { id: "later", label: "priorities.filter.later" },
];

export const matchesTurn = (item: WorkItem, filter: TurnFilter): boolean =>
  filter === "all" || (isInFlight(item) && item.turn === filter);
