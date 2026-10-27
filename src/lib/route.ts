// Edits go to the machine that holds the record: `node` is set on records from another machine.
import { api } from "./api";
import type { CommitmentInput, ItemDetail, ItemInput, Schedule, Step, TodoEdit } from "./types";

const peer = <T = unknown>(node: string, tool: string, args: object): Promise<T> => api.peerCall<T>(node, tool, args);

export const route = {
  runLog: (node: string | undefined, id: number) =>
    node ? peer<Step[]>(node, "get_run_log", { run_id: id }) : api.runLog(id),
  answerAsk: (node: string | undefined, id: number, text: string) =>
    node ? peer(node, "answer_ask", { ask_id: id, answer: text }) : api.respond({ kind: "answer", askId: id, text }),
  dismissAsk: (node: string | undefined, id: number) =>
    node ? peer(node, "dismiss_ask", { ask_id: id }) : api.respond({ kind: "dismiss", askId: id }),
  tellClaude: (node: string | undefined, itemId: string, text: string) =>
    node ? peer(node, "tell_claude", { item_id: itemId, text }) : api.respond({ kind: "tell", itemId, text }),
  setTodoDone: (node: string | undefined, id: number, done: boolean) =>
    node ? peer(node, "complete_todo", { id, done }) : api.setTodoDone(id, done),
  addTodo: (node: string | undefined, title: string, area: string, due: string | null, link: string | null) =>
    node ? peer(node, "add_todo", { title, area, due, link }) : api.addTodo(title, area, due, link),
  updateTodo: (node: string | undefined, todo: TodoEdit) =>
    node
      ? peer(node, "update_todo", { ...todo, due: todo.due ?? undefined, clear_due: todo.due === null, link: todo.link ?? "" })
      : api.updateTodo(todo),
  deleteTodo: (node: string | undefined, id: number) =>
    node ? peer(node, "delete_todo", { id }) : api.deleteTodo(id),
  hideMeeting: (node: string | undefined, id: number) =>
    node ? peer(node, "update_meeting", { id, hidden: true }) : api.hideCommitment(id),
  dismissUrgent: (node: string | undefined, id: number) =>
    node ? peer(node, "dismiss_urgent", { id }) : api.dismissUrgent(id),
  retryUrgent: (node: string | undefined, id: number) =>
    node ? peer(node, "retry_urgent", { id }) : api.retryUrgent(id),
  runSchedule: (node: string | undefined, id: string) =>
    node ? peer(node, "run_schedule", { id }) : api.runScheduleNow(id),
  setScheduleEnabled: (node: string | undefined, id: string, enabled: boolean) =>
    node ? peer(node, "set_schedule_enabled", { id, enabled }) : api.setScheduleEnabled(id, enabled),
  deleteSchedule: (node: string | undefined, id: string) =>
    node ? peer(node, "delete_schedule", { id }) : api.deleteSchedule(id),
  itemDetail: (node: string | undefined, id: string) =>
    node ? peer<ItemDetail>(node, "get_item", { id }) : api.itemDetail(id),
  park: (node: string | undefined, id: string, until: string, why: string | null) =>
    node ? peer(node, "park_item", { id, until, why }) : api.parkItem(id, until, why),
  unpark: (node: string | undefined, id: string) =>
    node ? peer(node, "park_item", { id, parked: false }) : api.unparkItem(id),
  deleteItem: (node: string | undefined, id: string) =>
    node ? peer(node, "delete_item", { id }) : api.deleteItem(id),
  addMeeting: (node: string | undefined, c: CommitmentInput) =>
    node
      ? peer(node, "add_meeting", { title: c.title, area: c.area, starts_at: c.startsAt, ends_at: c.endsAt, location: c.location })
      : api.addCommitment(c),
  approveScript: (node: string | undefined, id: string) =>
    node ? peer(node, "approve_script", { id }) : api.approveScript(id),
  saveSchedule: async (node: string | undefined, s: Schedule): Promise<unknown> => {
    if (!node || !s.id) return api.saveSchedule(s);
    await peer(node, "update_schedule", {
      id: s.id,
      name: s.name,
      area: s.area,
      instructions: s.prompt,
      allowed_tools: s.allowedTools,
      triggers: s.triggers,
      script: s.script,
      remove_script: !s.script,
    });
    // The user wrote it here, as when saving on this machine.
    return s.script ? peer(node, "approve_script", { id: s.id }) : undefined;
  },
  moveItem: (node: string, id: string, position: number) => peer(node, "move_item", { id, position }),
  saveItem: (node: string | undefined, item: ItemInput): Promise<string> => {
    if (!node) return api.saveItem(item);
    const fields = {
      title: item.title,
      area: item.area,
      stage: item.stage,
      state_label: item.stateLabel,
      link: item.link,
    };
    return item.id
      ? peer<string>(node, "update_item", { ...fields, id: item.id, status: item.status, clear_status: item.status === null })
      : peer<string>(node, "create_item", { ...fields, status: item.status });
  },
};
