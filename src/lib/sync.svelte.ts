import { listen } from "@tauri-apps/api/event";

import { api } from "./api";

/** Whether a GitHub sync is running, whoever started it. */
export const github = $state({ syncing: false });

/** Sent by the backend as a sync starts and as it lands. */
const SYNCING = "syncing";
const SYNCED = "synced";

/** Follows GitHub syncs until the returned cleanup runs. */
export function followSync(): () => void {
  void api.githubSyncing().then((running) => (github.syncing = running), () => undefined);
  const stops = [listen(SYNCING, () => (github.syncing = true)), listen(SYNCED, () => (github.syncing = false))];
  return () => {
    for (const stop of stops) void stop.then((s) => s());
  };
}
