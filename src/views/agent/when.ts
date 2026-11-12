import { hasCopy, t } from "../../lib/copy";
import type { Explanation, Trigger } from "../../lib/types";

/** An event's plain-English name from the copy file; custom events show their own name. */
export function eventText(on: string): string {
  const key = `event.${on}`;
  return hasCopy(key) ? t(key) : on;
}

function filterText(when: Record<string, string> | undefined): string {
  const pairs = Object.entries(when ?? {}).map(([k, v]) => `${k} ${v}`);
  return pairs.length > 0 ? t("schedule.filtered", { filters: pairs.join(", ") }) : "";
}

/** Plain-English "when" for a schedule's triggers. Only the first cron trigger is explained. */
export function whenText(triggers: Trigger[], explanation: Explanation | null): string {
  if (triggers.length === 0) return t("schedule.byHandOnly");
  const firstCron = triggers.findIndex((tr) => tr.type === "cron");
  return triggers
    .map((tr, i) => {
      if (tr.type === "event") return eventText(tr.on) + filterText(tr.when);
      return i === firstCron ? (explanation?.when ?? tr.expr) : tr.expr;
    })
    .join(t("schedule.or"));
}
