import { t } from "./copy";
import { dayClock } from "./time";
import type { Turn, WorkItem } from "./types";

/** Whose turn an item is, in a few words: "your turn", "waiting on CI". Empty for done work, which
 * has no turn; every open item, started or not, has one. */
export function turnText(item: Pick<WorkItem, "turn" | "waitingOn" | "stage">): string {
  if (item.stage === "done") return "";
  // An older peer's items carry no turn.
  const turn = item.turn ?? "you";
  if (turn === "others") return t("turn.others", { who: item.waitingOn ?? t("turn.someone") });
  return t(`turn.${turn}`);
}

/** The turn and when it runs out, e.g. "your agent's turn until 15:30". */
export function turnWithClock(item: WorkItem, now: Date): string {
  const text = turnText(item);
  return item.turnUntil ? t("turn.until", { turn: text, at: dayClock(item.turnUntil, now) }) : text;
}

/** A history line for a pass: "Your turn: approved, ready to merge". */
export const passText = (turn: Turn, why: string): string => t(`history.turn.${turn}`, { why });
