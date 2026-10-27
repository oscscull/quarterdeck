import { invoke } from "@tauri-apps/api/core";

import type {
  Response,
  TodoEdit,
  Calendar,
  MacCalendar,
  ClaudeConnection,
  AgentView,
  EventKindInfo,
  LiveRun,
  Step,
  Area,
  Commitment,
  CommitmentInput,
  Explanation,
  ItemDetail,
  ItemInput,
  Schedule,
  TodayView,
  WorkItem,
  ReplyOutcome,
  ChatMessage,
} from "./types";

/** Typed wrappers around the backend's Tauri commands. */
export const api = {
  machine: () => invoke<string>("machine"),
  peers: () => invoke<{ name: string; error: string | null }[]>("peers"),
  peerCall: <T = unknown>(node: string, tool: string, args: object) =>
    invoke<T>("peer_call", { node, tool, args }),
  claudeConnection: () => invoke<ClaudeConnection>("claude_connection"),
  calendars: () => invoke<Calendar[]>("calendars"),
  addCalendar: (url: string, area: string) => invoke<undefined>("add_calendar", { url, area }),
  /** `null` where there is no Calendar app (Windows, Linux). */
  macCalendars: () => invoke<MacCalendar[] | null>("mac_calendars"),
  addMacCalendar: (calendar: string, area: string) =>
    invoke<undefined>("add_mac_calendar", { calendar, area }),
  removeCalendar: (name: string) => invoke<undefined>("remove_calendar", { name }),
  dueAlert: () => invoke<Commitment | null>("due_alert"),
  alertDone: (id: number) => invoke<undefined>("alert_done", { id }),
  alertSnooze: (id: number) => invoke<undefined>("alert_snooze", { id }),
  areas: () => invoke<Area[]>("areas"),
  addTodo: (title: string, area: string, due: string | null, link: string | null) =>
    invoke<undefined>("add_todo", { todo: { title, area, due, link } }),
  setTodoDone: (id: number, done: boolean) => invoke<undefined>("set_todo_done", { id, done }),
  updateTodo: (todo: TodoEdit) => invoke<undefined>("update_todo", { todo }),
  deleteTodo: (id: number) => invoke<undefined>("delete_todo", { id }),
  today: () => invoke<TodayView>("today"),
  workItems: () => invoke<WorkItem[]>("work_items"),
  agent: () => invoke<AgentView>("agent"),
  liveRuns: () => invoke<LiveRun[]>("live_runs"),
  runLog: (id: number) => invoke<Step[]>("run_log", { id }),
  saveItem: (item: ItemInput) => invoke<string>("save_item", { item }),
  deleteItem: (id: string) => invoke<undefined>("delete_item", { id }),
  itemDetail: (id: string) => invoke<ItemDetail>("item_detail", { id }),
  parkItem: (id: string, until: string, why: string | null) => invoke<undefined>("park_item", { id, until, why }),
  unparkItem: (id: string) => invoke<undefined>("unpark_item", { id }),
  parkedItems: () => invoke<WorkItem[]>("parked_items"),
  respond: (response: Response) => invoke<ReplyOutcome>("respond", { response }),
  addCommitment: (commitment: CommitmentInput) => invoke<undefined>("add_commitment", { commitment }),
  hideCommitment: (id: number) => invoke<undefined>("hide_commitment", { id }),
  deleteCommitment: (id: number) => invoke<undefined>("delete_commitment", { id }),
  reorderItems: (ids: string[]) => invoke<undefined>("reorder_items", { ids }),
  setScheduleEnabled: (id: string, enabled: boolean) =>
    invoke<undefined>("set_schedule_enabled", { id, enabled }),
  saveSchedule: (schedule: Schedule) => invoke<string>("save_schedule", { schedule }),
  deleteSchedule: (id: string) => invoke<undefined>("delete_schedule", { id }),
  explainCron: (expr: string) => invoke<Explanation>("explain_cron", { expr }),
  eventKinds: () => invoke<EventKindInfo[]>("event_kinds"),
  approveScript: (id: string) => invoke<undefined>("approve_script", { id }),
  runScheduleNow: (id: string) => invoke<undefined>("run_schedule_now", { id }),
  pauseAll: (until: string | null) => invoke<undefined>("pause_all", { until }),
  resumeAll: () => invoke<undefined>("resume_all"),
  dismissUrgent: (id: number) => invoke<undefined>("dismiss_urgent", { id }),
  dismissAllUrgent: () => invoke<undefined>("dismiss_all_urgent"),
  retryUrgent: (id: number) => invoke<undefined>("retry_urgent", { id }),
  resolveUrgent: (id: number, message: string) => invoke<string>("resolve_urgent", { id, message }),
  askClaude: (page: string, context: string, history: ChatMessage[], message: string) =>
    invoke<string>("ask_claude", { request: { page, context, history, message } }),
  draftSchedule: (request: string) => invoke<string>("draft_schedule", { request }),
};

/** Backend errors arrive as plain strings. */
export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
