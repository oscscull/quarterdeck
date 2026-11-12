import { api, errorText } from "./api";
import { fromPeers, tag } from "./peers.svelte";
import type { WorkItem } from "./types";

/** Work items in priority order from every machine, shared by Priorities and Board. */
export function workItems() {
  const state = $state({ items: [] as WorkItem[], error: "", now: new Date() });

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([
        api.workItems(),
        fromPeers<WorkItem[]>("list_items", { include_done: true }),
      ]);
      // A peer on an older build sends no turn; its items show as the user's.
      const peerItems = remote.flatMap((r) => tag(r.value, r.node)).map((i) => ({ ...i, turn: i.turn ?? "you" }));
      state.items = [...local, ...peerItems];
      state.now = new Date();
      state.error = "";
    } catch (e) {
      state.error = errorText(e);
    }
  }

  return { state, load };
}

/** When parked work comes back; one with no time sorts last. */
const backAt = (i: WorkItem): number => (i.turnUntil ? new Date(i.turnUntil).getTime() : Infinity);

/** Parked items from every machine, soonest back first. */
export function parkedItems() {
  const state = $state({ items: [] as WorkItem[], error: "", now: new Date() });

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([
        api.parkedItems(),
        fromPeers<WorkItem[]>("list_items", { parked: true, include_done: true }),
      ]);
      state.items = [...local, ...remote.flatMap((r) => tag(r.value, r.node))].sort((a, b) => backAt(a) - backAt(b));
      state.now = new Date();
      state.error = "";
    } catch (e) {
      state.error = errorText(e);
    }
  }

  return { state, load };
}
