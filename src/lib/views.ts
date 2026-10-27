import type { CopyKey } from "./copy";

export type ViewId = "today" | "priorities" | "board" | "agent";

export const VIEWS: readonly ViewId[] = ["today", "priorities", "board", "agent"];

export const viewLabel = (view: ViewId): CopyKey => `view.${view}`;

export const suggestionKeys = (view: ViewId): CopyKey[] => [
  `chat.suggest.${view}.1`,
  `chat.suggest.${view}.2`,
];
