// Combines this machine's views with other machines', tagging every remote record.
import { tag } from "./peers.svelte";
import type { AgentView, ScheduleView, TodayView } from "./types";

interface Remote<T> {
  node: string;
  value: T;
}

const byTime = <T>(key: (x: T) => string, desc = false) => (a: T, b: T): number =>
  desc ? key(b).localeCompare(key(a)) : key(a).localeCompare(key(b));

export function mergeToday(local: TodayView, remote: Remote<TodayView>[]): TodayView {
  const all = (pick: (v: TodayView) => object[], r: Remote<TodayView>) => tag(pick(r.value), r.node);
  return {
    ...local,
    upcoming: [...local.upcoming, ...remote.flatMap((r) => all((v) => v.upcoming, r))].sort(
      byTime((c) => (c as { startsAt: string }).startsAt),
    ) as TodayView["upcoming"],
    priorities: [...local.priorities, ...remote.flatMap((r) => tag(r.value.priorities, r.node))],
    todos: [...local.todos, ...remote.flatMap((r) => tag(r.value.todos, r.node))],
    triage: [...(local.triage ?? []), ...remote.flatMap((r) => tag(r.value.triage ?? [], r.node))],
    urgent: [...local.urgent, ...remote.flatMap((r) => tag(r.value.urgent, r.node))],
    activity: [...local.activity, ...remote.flatMap((r) => tag(r.value.activity, r.node))].sort(
      byTime((a) => a.at, true),
    ),
    running: local.running + remote.reduce((n, r) => n + r.value.running, 0),
    inFlight: local.inFlight + remote.reduce((n, r) => n + r.value.inFlight, 0),
  };
}

export function mergeAgent(local: AgentView, remote: Remote<AgentView>[]): AgentView {
  const schedules: ScheduleView[] = [
    ...local.schedules,
    ...remote.flatMap((r) => tag(r.value.schedules, r.node)),
  ];
  return {
    ...local,
    schedules,
    live: [...local.live, ...remote.flatMap((r) => r.value.live)],
    // An older peer sends no turns or events.
    turns: [...local.turns, ...remote.flatMap((r) => tag(r.value.turns ?? [], r.node))],
    events: [...(local.events ?? []), ...remote.flatMap((r) => tag(r.value.events ?? [], r.node))].sort(
      byTime((e) => e.at, true),
    ),
  };
}
