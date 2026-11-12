import { ALL, areas } from "../../lib/areas.svelte";
import { t } from "../../lib/copy";
import type { Schedule } from "../../lib/types";

/** A new schedule for "Set it up yourself". An empty id tells the backend to create a file. */
export function blankSchedule(): Schedule {
  return {
    id: "",
    name: t("schedule.blankName"),
    prompt: "",
    script: null,
    triggers: [{ type: "cron", expr: "0 9 * * 1-5" }],
    area: areas.current !== ALL ? areas.current : (areas.list[0]?.id ?? ""),
    enabled: true,
    request: "",
    allowedTools: [],
  };
}
