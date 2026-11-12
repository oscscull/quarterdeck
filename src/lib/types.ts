// Mirrors the Rust types the backend sends. Keep in step with src-tauri/src/domain and views.

export type Stage = "up_next" | "in_progress" | "in_review" | "done";
/** `decision` is retired; older peers may still send it. */
export type Status = "decision" | "review" | "ci" | "ci_failed" | "dependency" | "approved" | "queued";
/** Whose move an item is. Started work always has one, with a deadline; up next and done work has none. */
export type Turn = "you" | "agent" | "others" | "later";

/** An event that wakes parked work, in the schedules' trigger format. */
export interface OnEvent {
  on: string;
  when?: Record<string, string>;
}

export interface WorkItem {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: string;
  title: string;
  stateLabel: string;
  stage: Stage;
  status: Status | null;
  /** Null while up next or done. */
  turn: Turn | null;
  /** When the turn runs out. */
  turnUntil: string | null;
  /** On the user's turn: what's needed from them. On a later turn: why it waits, and what then. */
  need?: string | null;
  /** On a later (parked) turn: events that bring it back sooner. */
  wake?: OnEvent[];
  /** Who it waits on while it's others' turn, e.g. reviewers or "CI". */
  waitingOn: string | null;
  /** Who gets the turn when others' or a later turn runs out: you or the agent. */
  thenTurn: Turn | null;
  area: string;
  link: string | null;
  /** "manual", or the connector that made it, e.g. "github". */
  source: string;
  externalId: string | null;
  openedAt: string;
  stateSince: string;
  rank: number;
}

/** What parking sends: why, and when it's back (RFC 3339) and to whom, or no date when it waits for
 * something to happen, which the agent checks. */
export interface Parking {
  until: string | null;
  why: string | null;
  then: "you" | "agent";
}

/** What the UI sends to create (empty id) or edit an item. */
export interface ItemInput {
  id: string;
  title: string;
  stateLabel: string;
  stage: Stage;
  status: Status | null;
  area: string;
  link: string | null;
}

export type Change =
  | { kind: "created" }
  | { kind: "stage"; stage: Stage }
  | { kind: "status"; status: Status | null }
  | { kind: "label"; label: string }
  | { kind: "note"; text: string }
  | { kind: "turn"; turn: Turn; why: string }
  | { kind: "record"; tag: RecordTag; text: string };

export type RecordTag = "why" | "decision" | "change" | "blocked" | "correction" | "delivered";

export interface ItemEvent {
  id: number;
  itemId: string;
  at: string;
  change: Change;
  /** Bookkeeping rather than record. Missing from older peers. */
  trace?: boolean;
}

export type ProjectState = "moving" | "stalled" | "quiet" | "done";

/** One paragraph of a briefing. Citations are `[ref:<id>]` markers in `text`. */
export interface Paragraph {
  project: string | null;
  text: string;
}

export interface Briefing {
  done: Paragraph[];
  now: Paragraph[];
  next: Paragraph[];
}

export interface AreaBriefing {
  node?: string;
  area: string;
  briefing: Briefing;
  writtenAt: string;
}

export type RefKind = "you_said" | "ticket" | "pr" | "comment" | "slack" | "doc" | "commit" | "config" | "other";

export interface Reference {
  node?: string;
  id: number;
  area: string;
  kind: RefKind;
  title: string;
  url: string | null;
  quote: string;
  by: "you" | "agent";
  at: string;
  project: string | null;
  items: string[];
}

export interface ProjectView {
  node?: string;
  id: string;
  title: string;
  aim: string;
  area: string;
  state: ProjectState;
  rank: number;
  openedAt: string;
  stateSince: string;
  briefing: Briefing | null;
  briefedAt: string | null;
  items: WorkItem[];
}

/** One message from the Briefing's correction box, with the conversation so far. */
export interface Correction {
  /** What the user is looking at, in words, e.g. "the Approvals project". */
  scope: string;
  area: string | null;
  project: string | null;
  history: ChatMessage[];
  message: string;
}

export interface BriefingView {
  areas: AreaBriefing[];
  projects: ProjectView[];
  references: Reference[];
  /** Areas whose briefing is being rewritten right now. */
  writing: string[];
}

export interface ItemDetail {
  item: WorkItem;
  asks: Ask[];
  history: ItemEvent[];
}

export interface CommitmentInput {
  title: string;
  area: string;
  startsAt: string;
  endsAt: string;
  location: string | null;
}

export interface Ask {
  id: number;
  itemId: string;
  question: string;
  context: string;
  /** Choices to answer with. Empty means a written answer; one can always be written. */
  options: string[];
}

/** Something that needs the user, derived from the work. `kind` says why. */
export type Priority = (
  | { kind: "ask"; ask: Ask }
  /** `need` is missing from machines running an older Quarterdeck. */
  | { kind: "yours"; need?: string }
) & {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  itemId: string;
  itemTitle: string;
  itemLabel: string;
  area: string;
  since: string;
  /** The item's ticket or PR. */
  link: string | null;
};

export interface Commitment {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: number;
  title: string;
  area: string;
  startsAt: string;
  endsAt: string;
  location: string | null;
  joinUrl: string | null;
  source: string;
  externalId: string | null;
  hidden: boolean;
}

export interface Activity {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: number;
  text: string;
  area: string;
  at: string;
  unread: boolean;
}

/** A part of life from quarterdeck.toml, e.g. Work or Life. */
export interface Area {
  id: string;
  name: string;
  color: string;
  node: string;
}

export interface Todo {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: number;
  title: string;
  area: string;
  /** A calendar date, "YYYY-MM-DD". */
  due: string | null;
  /** A page it's about, e.g. a booking form. */
  link: string | null;
  doneAt: string | null;
}

export interface Urgent {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: number;
  title: string;
  detail: string;
  area: string;
  raisedAt: string;
  scheduleId: string | null;
  /** Set when it's about a failed agent run, which can be retried. Missing from older machines. */
  runId?: number | null;
}

/** A GitHub issue nobody has picked up yet. */
export interface Triage {
  itemId: string;
  title: string;
  area: string;
  node?: string;
}

/** The agent can't run at all; its work waits until a check finds it back. */
export interface Outage {
  why: string;
  since: string;
  nextCheck: string | null;
}

export interface TodayView {
  /** Missing from machines running an older Quarterdeck. */
  outage?: Outage | null;
  urgent: Urgent[];
  upcoming: Commitment[];
  priorities: Priority[];
  priorityLimit: number;
  todos: Todo[];
  /** Missing from machines running an older Quarterdeck. */
  triage?: Triage[];
  activity: Activity[];
  running: number;
  inFlight: number;
}

export type Outcome = "ok" | "failed" | "missed";

export interface Run {
  id: number;
  scheduleId: string | null;
  what: string;
  target: string;
  node: string;
  startedAt: string;
  finishedAt: string | null;
  outcome: Outcome | null;
  summary: string | null;
}

/** Starts a schedule when an event happens. `when` filters on its fields; `*` is a wildcard. */
export interface OnEvent {
  on: string;
  when?: Record<string, string>;
  /** Minutes between runs on the same subject. */
  cooldown?: number;
}
export type Trigger = { type: "cron"; expr: string } | ({ type: "event" } & OnEvent);

export type Report = "always" | "failures" | "never";

/** How a script schedule runs. */
export interface Script {
  cwd?: string | null;
  timeout_minutes: number;
  report: Report;
}

/** An event Quarterdeck fires, with the fields a trigger can filter on. */
export interface EventKindInfo {
  name: string;
  about: string;
  fields: string[];
}

/** Something that happened, and the schedules it started. */
export interface FiredEvent {
  /** Set when it happened on another machine. */
  node?: string;
  id: number;
  kind: string;
  area: string;
  subject: string;
  data: Record<string, string>;
  about: string;
  depth: number;
  at: string;
  started: string[];
}

export interface Schedule {
  /** Set when it lives on another machine; edits go back there. */
  node?: string;
  id: string;
  name: string;
  prompt: string;
  /** Set for a script schedule: `prompt` is the script, run in the user's shell. */
  script: Script | null;
  /** Any of these starts it; none means it only runs when run by hand. */
  triggers: Trigger[];
  area: string;
  enabled: boolean;
  request: string;
  allowedTools: string[];
}

export type Unit = "minute" | "hour" | "day" | "month" | "weekday";

export interface CronField {
  unit: Unit;
  value: string;
  means: string;
}

export interface Explanation {
  when: string;
  fields: CronField[];
  nextRuns: string[];
}

export interface ScheduleView extends Schedule {
  explanation: Explanation | null;
  runs: Run[];
  /** A script waiting for the user to approve it as it stands. Older peers don't say. */
  needsApproval?: boolean;
}

export type StepKind = "doing" | "tool" | "said" | "failed" | "output";

/** One step of a live run. `tool` is set for tool calls. */
export interface Step {
  at: string;
  kind: StepKind;
  tool: string | null;
  text: string;
}

/** A run going now, with the item it works on and its latest step. */
export interface LiveRun extends Run {
  title: string | null;
  now: Step | null;
}

/** An item that is the agent's turn, and where that stands. */
export interface AgentTurn {
  node?: string;
  itemId: string;
  title: string;
  area: string;
  doing: "running" | "session" | "starting";
  until: string | null;
}

export interface AgentView {
  live: LiveRun[];
  turns: AgentTurn[];
  schedules: ScheduleView[];
  /** Newest first. Older peers send none. */
  events?: FiredEvent[];
  problems: string[];
  /** Set while every schedule is paused. `until: null` means until resumed. */
  pause: { until: string | null } | null;
  connections: Connection[];
}

export interface Connection {
  name: string;
  lastSync: string | null;
  error: string | null;
}

/** Whether the user's own Claude Code sessions can reach Quarterdeck. */
export type ClaudeConnection =
  | { state: "connected" }
  | { state: "missing"; command: string }
  | { state: "elsewhere"; command: string };

/** A synced calendar. Its private link stays in quarterdeck.toml. */
export interface Calendar {
  name: string;
  area: string;
}

/** A calendar in the macOS Calendar app, and the account it belongs to. */
export interface MacCalendar {
  title: string;
  account: string;
}

/** A to-do as the edit form saves it. `due: null` clears the date. */
export interface TodoEdit {
  id: number;
  title: string;
  area: string;
  due: string | null;
  link: string | null;
}

/** What the user does about something that needs them. */
export type Response =
  | { kind: "answer"; askId: number; text: string }
  | { kind: "dismiss"; askId: number }
  | { kind: "tell"; itemId: string; text: string };

/** What came of a reply: "gone" when the question was already answered or dismissed. */
export type ReplyOutcome = "done" | "gone";

/** One message in the "Ask Claude" chat. */
export interface ChatMessage {
  from: "you" | "claude";
  /** The view it was sent from, so Claude knows when the user moved on. */
  page: string;
  text: string;
}
