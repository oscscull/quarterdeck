# Quarterdeck spec

Reader: anyone (human or agent) changing Quarterdeck. Rewrite this when the design changes. Setup lives
in [setup.md](setup.md).

## What it is

A calm desktop app that holds the state and the clock for work and life done with Claude. Claude
does the judgement work; Quarterdeck shows what's in flight, runs recurring jobs, and interrupts only
when needed. Natural language is the main way in: the MCP server and Quarterdeck's own chat change
anything the UI shows.

## Hierarchy of attention

1. **Fixed commitments** (meetings): the loudest thing. An on-top countdown with a chime opens a
   few minutes before each.
2. **Priorities**: things where the user's attention unblocks work (design forks, questions).
3. **Claude's activity**: quiet, notification-style updates.

Urgent items (something went wrong or can't wait) show as a red pill next to the next meeting.

## Areas and machines

Everything belongs to an area (Work, Projects, Life by default), defined in `quarterdeck.toml` with the
machine (`node`) that runs it. The switcher filters every view. Each machine keeps its own records
and serves them to peers; the UI merges peers' data and sends edits back to the record's machine.
Connectors and schedules only act for areas this machine runs.

The areas list is the one thing every machine shares. Each remembers, per peer, the list they last
agreed on; when its own differs (a tool or a hand edit) it pushes it each scheduler tick until the
peer takes it. A peer takes it only if its own list hasn't changed since they agreed; if both
changed, it's raised as urgent for the user to settle. Once two machines agree, a schedule held on
one but owned by the other (its area was edited, or the area moved) is handed over: created there,
then deleted here. Run now on such a schedule hands it over and runs it there.

The window title shows the running version, so it's clear when an update has landed.

## Views

- **Today**: next meeting, Priorities (in rank order: questions on an item, else what the user's
  turn on it needs; the first `priority_limit` show), one line for GitHub issues nobody has picked up yet
  (opens to the list), To do (things only the user can do, each with an optional link), and "From
  Claude".
- **Priorities**: everything in play, drag to rank (per machine). The ranking drives Today.
- **Board**: items by stage, with parked items in a list of their own below. **In flight**: open
  items by whose turn it is.
- **An item** opens with what's needed from the user when it's their turn (the need, or the
  questions asked), the reply box, then its state, links and (collapsed) history.
- **Agent**: every item that is the agent's turn, and what's running (each run's latest step, live; click for its steps so far), every schedule (toggle, edit, delete, run now, create by describing
  it), missed or failed runs, Pause all, recent events and what they started, connector and peer
  status, and the MCP connect command.
- **Ask Claude** bubble on every view, given that view's data and the Quarterdeck tools, settings
  ones included, so any setting can be changed by asking. One conversation runs across views until
  the app restarts (never stored); each message carries the last `chat_history_messages` with the
  view each was sent from. In it and in Tell Claude, Enter sends and Cmd/Ctrl/Shift+Enter adds a line.

## Item life

Every item is in exactly one place, decided in one function (`attention`) that Today and the turn
clock both read, and finishing or deleting it is the only way off Today for good:

- **Up next** (unstarted): the backlog. Nobody's turn, no clock, off Today; GitHub issues nobody has
  picked up are counted on Today's triage line. Giving it a turn (a session claims it, Tell
  Claude, the queue, a question) starts it.
- **In progress / in review** (started): always someone's turn with a clock (see Turns). The
  user's turn is on Today, saying what it needs; anyone else's comes back when its time runs out.
  Started work never goes back to up next: it's parked or closed.
- **Done**: no turn.

The type (`Life` in `domain/life.rs`) makes other states unrepresentable: started work without a
turn reads as the user's, and the user's turn always carries a need. A test walks every state
forward through its clock and checks each ends on Today.

**Parking** puts started work off until something happens: a time, events (triggers in the
schedules' format, e.g. another item's `item.stage` to done), or both, whichever comes first. It's
a `later` turn, so it leaves Today and the board's columns and comes back as the user's turn. With
only events, `later_hours` is the backstop. Nothing parks for good: work nobody will pick up is
closed or deleted.

## Where things live

- **Config folder** `~/.config/quarterdeck/`, editable by people and agents, re-read on each refresh:
  `quarterdeck.toml` (settings, areas, connectors, sync; plain settings change by name with
  `set_setting`, which leaves out `claude_command`), `prompts/*.md` (Quarterdeck's own Claude jobs),
  `schedules/<id>.md` (TOML front matter + instructions), `inbox/` (agent-raised urgent items,
  ingested and deleted), `pause.toml` (only while paused). Tools edit `quarterdeck.toml` in place,
  keeping comments, and roll back edits that would break it.
- **Database** (`quarterdeck.db`, SQLite through [Based](https://github.com/oscscull/baseddsl)): live
  state only. Tables and every query are declared in `src-tauri/schema/`; migrations in
  `src-tauri/migrations/` are built into the binary and applied when the app or MCP server opens
  the database. Activity, finished runs, done to-dos, done items with their history, and past
  meetings all expire. The first start of a Based build copies a pre-Based `quarterdeck.sqlite`
  across once and keeps it as `quarterdeck.sqlite.imported`.
- **Dev builds** (debug, `npm run tauri dev`) are a separate app: id `com.oscscull.quarterdeck.dev`,
  config `~/.config/quarterdeck-dev/`, their own data folder, MCP name `quarterdeck-dev`, a banner on
  every screen, no updater, and no Claude runs or scripts unless their `[dev] run_claude` or
  `run_scripts` is on.
  `npm run dev:seed` replaces a dev build's config and data with a snapshot of the installed app's,
  paused and with sync and peers off.

## Connectors

Populate by default, never fight overrides. Each keeps the last snapshot it saw and only re-applies
the fields it owns when they change upstream; hidden or re-filed records stay that way.

- **GitHub** (`gh` CLI): open PRs become items; new comments, CI failing and CI passing fire events.
  An open hand-made item whose link is a PR becomes that PR's item, replacing any the sync made.
  CI state is worked out per check: checks a repo marks informational (`informational_checks`, e.g.
  a visual-diff report) don't count, and their results are noted on the item instead.
  PRs with no requested reviewers, or in `self_review` repos, wait on the user. Every sync also reads
  each PR's merge state, so work waiting on review stays mergeable: one that falls behind a base it
  must be up to date with fires `pr_behind` and Quarterdeck updates its branch (`gh pr update-branch`,
  `update_behind`; on failure the turn goes to the user); one that starts conflicting fires
  `pr_conflicts` and goes to Claude like a CI failure (`prompts/fix-conflicts.md`, `fix_conflicts`).
  A schedule on either event takes over. An item is closed
  only when `gh` says its PR merged or closed, never because the open-PR search missed it. Syncs
  every `poll_minutes`, and at once when the window regains focus (at most every
  `focus_sync_seconds`), after which the screen reloads.
- **GitHub issues**: open issues of the owners and repos in `[github.issues] watch` become items to
  triage (up next, no holder), filed by `[github.areas]` like PRs and synced by the machine that
  runs that area, skipping `skip_labels`. No agent is involved. An issue one of the user's open PRs
  closes is left to that PR's item, which notes it; if that PR closes unmerged, the issue comes
  back. An item is closed only when `gh` says its issue closed, and reopened if the issue reopens.
  An agent's hand-made item linking the issue becomes the synced one. Since the item links the
  issue, queueing it for Claude works as for any issue item.
- **Calendar**: meetings, with recurrence, exceptions and join links. A feed is either a private iCal
  link or a calendar in the macOS Calendar app, for accounts that offer no link. A feed that can't
  be fetched or isn't a whole calendar is an error, never an empty calendar. The Calendar app is
  read by `quarterdeck-calendars`, a Swift EventKit helper (`src-tauri/helpers/`) that `build.rs`
  compiles on macOS and the app ships as a sidecar, since the Rust code forbids `unsafe`. Only a
  Mac that runs a feed's area can read its Calendar app.
- **Slack**: no connector; agents bring things in through the tools.

## Running Claude

`claude -p` on the user's subscription, with the Quarterdeck MCP tools always allowed. It is found
in the usual install folders, else through the user's login shell (so nvm installs work); if it
can't be found, the error names the program and the setting to fix. Runs stream
(`--output-format stream-json`): each step goes to a small per-run file in the data folder's `live/`,
capped and deleted when the run ends, which the Agent tab and peers read (`list_live_runs`,
`get_run_log`). Schedules can use other tools only when their file lists them in `allowed_tools`.
The scheduler ticks every 30s:
cron runs fire on time, runs missed while asleep are recorded and raised as urgent, and Pause all
stops everything.

## Events and triggers

A schedule has a list of triggers, any of which starts it: cron expressions, and events. Events
cost nothing until they fire. Each is stored (kind, area, subject, string fields, a line for the
run) and the next tick starts the enabled schedules in that area, on the machine that runs it,
with a trigger for that kind whose `when` filters all match (`*` is a wildcard). The run is told
the event and runs against its subject (an item or schedule), never twice at once on one subject,
and not again within the trigger's `cooldown` minutes. Events fire from:

- GitHub syncs: `github.ci_failed`, `github.ci_passed`, `github.pr_new_comments`. CI results also
  reach claimed sessions and fix-ci runs as described under Turns.
- Item history: `item.created`, `item.stage`, `item.turn`. History copied from an old database
  fires nothing.

Events also wake parked items whose `wake` they match (an item's own turn changes never wake it),
even while paused.
- Runs: `run.finished` (with `outcome`) when a schedule's run ends, so schedules can chain.
- The scheduler itself: `meeting.starting` (when its countdown appears) and `meeting.ended`, and
  `file.changed` for a path a trigger names in `when.path` (checked each tick: a file's
  modification time and size, a folder's newest direct entry and count; the first look only
  records it). Moments the machine slept through by more than 10 minutes fire nothing.
- Agents and peers: `custom.<name>` through `emit_event`, sent to the machine that runs its area.

Each event counts how many led to it: a run started by an event passes its depth + 1 to the MCP
server it runs with (or a script's environment), so anything it changes is one step deeper. From depth 3 on, schedules
stop starting each other and it's raised as urgent. Events wait while the agent is down, and while paused they start nothing. Events
expire after `event_ttl_days`. `list_event_kinds` describes them all.

## Scripts

A schedule whose file has a `[script]` table runs its body in the user's login shell instead of
asking Claude: no tokens, for anything that needs no judgement. `cwd` (default home), and
`timeout_minutes` (default 10; it's killed after). Its output streams to the run log like a Claude
run's steps; exit 0 succeeds with its last line of output as the summary, anything else fails with
its last error line. `report` says what the user hears: `failures` (default; raised as urgent),
`always` (successes also in "From Claude"), or `never`. Every run is in the history and fires
`run.finished` either way. The environment says what started it (`QD_EVENT`, `QD_SUBJECT`,
`QD_EVENT_FIELDS`, `QD_EVENT_ABOUT`, plus `QD_SCHEDULE`, `QD_AREA`, `QD_NODE`, `QD_RUN_ID`) and
`QUARTERDECK` is the app binary: `"$QUARTERDECK" tool <name> '<json>'` runs any MCP tool and prints
its JSON result, so a script can fire `custom.*` events to wake a Claude schedule only when needed.

Agents can write schedule files and call every tool, so a script runs only once the user has
approved exactly that script in that folder (a SHA-256 of both, kept in the database). Saving in
the app's editor approves; anything else (a tool, a hand edit, a peer's `create_schedule`) leaves
it waiting, flagged on the Agent tab and raised once as urgent, until the user clicks Approve.
`approve_script` is a peer-only tool, never offered over MCP; a schedule handed to another machine
stays approved there if it was here.

## Turns

Every started item has a turn, `you` (with what's needed), `agent`, `others` (with who: reviewers,
CI) or `later` (with what wakes it), and a clock, so nothing idles. Turns change only through
`turns::pass`, which records why on the item's history and starts unstarted work; the UI and peers
read the stored turn and never derive it. A wait that runs out goes only to the user or the agent,
and the agent's runs out to the user, so every turn ends with the user.

- **The agent's turn**: Quarterdeck prompts it. Each tick, an agent's-turn item with nothing
  running gets a run (`prompts/continue.md`), resuming the session holding it or starting one; a
  session working outside Quarterdeck is left alone until it goes quiet for `quiet_minutes`. Every
  run on an item starts on the agent's turn and ends with it handed on: the agent records what it
  did with `pass_turn` (to itself, the user, or others), as `prompts/hand-over.md` tells every run.
  A run that ends without passing hands the turn to the user with its last message; a run that
  fails raises an urgent item with Retry (`retry_urgent`: back to the agent, whose next prompt is
  the failed run's own, kept on its run record) and hands it to the user. Passing to itself keeps
  the clock, so after `agent_hours` the turn comes back to the user regardless.
- **When the agent can't run at all** (offline, `claude` missing, API overloaded), its work waits
  instead of failing. Each tick checks first that `claude` exists and `[outage] probe` answers; a
  failed check, or a run whose error says it never reached the agent, raises one urgent item and
  stops prompting, delivering answers and starting the queue. Agent's-turn items keep the turn
  and an undelivered answer stays to go again. Checks repeat after `first_retry_seconds`, doubling
  to `max_retry_seconds`, and at once when the window regains focus; the first good one clears the
  item and gives waiting items a fresh `agent_hours`.
- **The user's turn**: says what's needed (`pass_turn` requires `need`; system passes use their
  reason) and is reminded every `you_remind_hours`. Any reply (an answer, Tell Claude)
  passes the turn to the agent, unless another question on the item is open; one that no session
  or worktree can take is raised as urgent (replacing the item's earlier one) and the turn comes
  back with the cause: no link, a link that isn't a GitHub repo or issue, or a repo with no project.
- **Others' turn**: after `others_hours` it goes to the item's `then`, else `others_then` (`you`
  by default; only `you` or `agent`). GitHub sets it from the PR: waiting on CI or reviewers is others', approved, draft or
  changes requested is the user's. GitHub never takes the turn from the agent.
- **Later**: comes back when its time does, or sooner when an event in its `wake` fires.

Limits live in `[turns]` (`set_turn_limits`).

CI is Quarterdeck's to watch, never an agent's. When CI finishes on an item a Claude session claimed
(`claim_item`), Quarterdeck resumes that session with the result (`prompts/ci-done.md`). A failure
nobody is on, and no `ci_failed` schedule covers, goes to a new claimed session
(`prompts/fix-ci.md`, `[github] fix_ci`).

Questions (`add_ask`) are answered, not ticked off: with one of the agent's `options` or in writing,
on Today or the item. Any open item, and every decision on Today, also takes a plain instruction
("Tell Claude what to do", `tell_claude`), delivered the same way (`prompts/message.md`). Delivery
follows on the next tick. The answer goes on the item's history, and the scheduler resumes the session
that asked (or the one holding the item) with it (`prompts/answer.md`). Sessions are stored with
their working directory, since Claude only resumes a session from the folder it ran in.

Work Quarterdeck starts on a PR (fixing CI, acting on an answer) or on an item whose link is a
GitHub issue or repo runs in that repo's project when `[[projects]]` maps it to a checkout
(`set_project`): in a git worktree per item under the data folder, in the permission mode
`project_permission_mode` sets (default `auto`, as in the user's own sessions), with the
project's `.claude/settings.local.json` copied in before every run. A PR's worktree is detached at its head; an
issue's is a new branch `issue-<n>` off the latest default branch, and the first run there also
gets the issue's text (`prompts/issue-workspace.md`); a repo's is a new branch
`quarterdeck/<item>` off it (`prompts/new-workspace.md`). The user's checkout is untouched. Since a
project's sandbox may keep Claude off GitHub, Quarterdeck does that side when a run finishes and no
question on the item is open: it pushes commits the branch doesn't have and, for an issue or repo,
opens a PR (closing the issue, titled from the first commit) and makes the item that PR's, in review, so syncs and
CI results follow it. A push that fails is raised as urgent and the turn goes to the user. Worktrees are
removed once their item is done. Repos with no project get a run that can read CI but not change
code.

`[queue]` lists items Claude works on one at a time (`set_queue`, `get_queue`). The first not yet
in review or done is current: when it's up next, Quarterdeck gives the agent the turn (which starts
it) and delivers `prompts/queue.md` to it like a message; the next waits until it reaches
review or done.

Default prompt files Quarterdeck wrote and nobody has edited follow newer defaults
(`.quarterdeck-defaults` records what it wrote); edited ones, and `quarterdeck.toml`, are left alone.

## Updates

Every merge to `main` builds macOS, Windows and Linux in CI and publishes a signed release. Each
installed app checks `releases/latest` every `[updates] check_minutes`, from the scheduler (no
Claude), and installs a new one only when no Claude run is live and the window isn't in use, then
restarts. So every machine runs the same build without rebuilding locally.

## Not built yet

- Slack as a native connector.
