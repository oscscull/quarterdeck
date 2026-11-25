import type { CopyKey } from "./copy";

export type ViewId = "today" | "briefing" | "priorities" | "board" | "archive" | "agent";

/** The tabs used every day, first in the nav. */
export const PRIMARY_VIEWS: readonly ViewId[] = ["today", "priorities", "board", "agent"];

/** Tabs visited now and then, shown after the primary ones and quieter. */
export const SECONDARY_VIEWS: readonly ViewId[] = ["briefing", "archive"];

export const viewLabel = (view: ViewId): CopyKey => `view.${view}`;

export const suggestionKeys = (view: ViewId): CopyKey[] => [
  `chat.suggest.${view}.1`,
  `chat.suggest.${view}.2`,
];
