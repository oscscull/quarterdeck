# Quarterdeck spec

Reader: anyone (human or agent) changing Quarterdeck. Rewrite this when the design changes. Setup lives
in [setup.md](setup.md), building it in [development.md](development.md).

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

The machine that holds a record keeps its clock, whatever area it's in: no other machine has it.
Each peer pass checks every peer and tells each one that answers where this machine is (`hello`,
peer-only); the receiver also notes the address the call came from. A peer whose configured URL
stops answering is tried at those, and one that answers as the right machine has its URL rewritten.
A peer unreachable for `[sync] down_minutes` is raised as urgent until it's back, since its records
can't be seen meanwhile. An area must run on this machine or a known peer.

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
  Claude". Under Priorities, one line counts parked work and says when the next is back; it opens
  Priorities.
- **Priorities**: everything in play, drag to rank (per machine). The ranking drives Today. Each row
  shows its stage, whose turn it is, its state and (once started) how long it has been open; a filter
  narrows the list to one side's turn, and ranking still works within it. Below, **Parked**: soonest
  back first, each with its note, when it's back and to whom, and a countdown.
- **Board**: items by stage; parked work is left out.
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

Nothing can be stuck: every open item, started or not, is someone's turn with a clock, and every
clock but the user's (who sees it on Today) runs out within a day. Where each item stands is decided
in one function (`attention`) that Today and the turn clock both read, and finishing or deleting it
is the only way off Today for good:

- **Open** (up next, in progress, in review): always someone's turn with a clock (see Turns). Up
  next is work not started yet, not a backlog: new work is the user's turn ("start it, park it
  until it can, or close it"), on Today; GitHub issues nobody has looked at are counted on Today's
  triage line instead. Giving the agent the turn on up-next work starts it. Moving open work
  between stages keeps its turn.
- **Done**: no turn. A question asked on done work still shows on Today.

The type (`Life` in `domain/life.rs`: `Open` or `Done`) makes other states unrepresentable: open
work stored without a turn reads as the user's, saying what it needs. A test walks every open state
forward through its clock and checks each ends on Today, with no step over a day unchecked.

**Parking** is the one way to put work off, and only work that can't start or go on yet: never
work that merely isn't due. It always says why (`why`), and is either:

- **until a date**: back then (or sooner, on a `wake` event) as the user's turn, or the agent's if
  the parking says so. Any date is fine: it's a concrete rule.
- **until something happens**: `wake` events (triggers in the schedules' format, e.g. another
  item's `item.stage` to done) bring it back as soon as they fire, and the agent looks again every
  `later_hours` (at most a day) regardless, starting it or parking it again.

It's a `later` turn, so it leaves Today and the board's columns for the Parked list, and its note
becomes the reason it came back, so the user's need or the agent's prompt says what to do. The
queue passes over parked up-next work. Nothing parks for good: work nobody will pick up is closed
or deleted.

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
  A schedule on either event takes over. New review comments go to Claude the same way
  (`prompts/address-review.md`, `fix_reviews`): it makes the clear changes, replies on each thread
  with the commit, and asks the user only about judgement calls. An item is closed
  only when `gh` says its PR merged or closed, never because the open-PR search missed it. Syncs
  every `poll_minutes`, and at once when the window regains focus (at most every
  `focus_sync_seconds`), after which the screen reloads.
- **GitHub issues**: open issues of the owners and repos in `[github.issues] watch` become items to
  triage (up next, the user's turn, counted on Today's triage line), filed by `[github.areas]` like PRs and synced by the machine that
  runs that area, skipping `skip_labels`. No agent is involved. An issue one of the user's open PRs
  closes is left to that PR's item (only one open and tracked here), which notes it; if that PR
  closes unmerged, the issue comes
  back. An item is closed only when `gh` says its issue closed, and reopened if the issue reopens.
  An agent's hand-made open item linking the issue becomes the synced one, whatever its id; a
  twin the sync made that nobody picked up goes. Since the item links the
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

Every open item has a turn, `you` (with what's needed), `agent`, `others` (with who: reviewers,
CI) or `later` (parked, with why), and a clock, so nothing idles. Turns change only through
`turns::pass`, which records why on the item's history; the UI and peers read the stored turn and
never derive it. The agent's and others' turns last at most `LONGEST_CHECK_HOURS` (24), whatever
the config or a caller asks; so does parking without a date. A pass from the agent to itself keeps
its clock, and so does others' turn changing only who it's on (a PR going from CI to reviewers), so
neither can renew itself. A wait that runs out goes only to the user or the agent, and the agent's
runs out to the user, so every turn ends with the user.

Clocks never stop. Each tick keeps them first, before anything else can fail; a pause or an agent
that can't run stops only prompting, so its turns still run out to the user. Ticks that keep
failing (e.g. a broken `quarterdeck.toml`) raise an urgent card after five minutes, and GitHub syncs
failing for `[github] failing_minutes` do too. Such cards come back if dismissed while the problem
lasts (`lasting`).

Nothing an agent or a sync sets up can stall an item: the agent always has somewhere to run, a
session it can't resume is dropped, and a failure is tried again by itself or waits for what it
needs (a connection) to come back. The user's turn means only that the user is needed.

- **The agent's turn**: Quarterdeck prompts it. Each tick, an agent's-turn item with nothing
  running gets a run (`prompts/continue.md`), resuming the session holding it or starting one: in
  its worktree, else in Quarterdeck's own folder, told why it has no checkout
  (`prompts/no-workspace.md`) so it links the item or asks where the repo is; a
  session working outside Quarterdeck is left alone until it goes quiet for `quiet_minutes`. Every
  run on an item starts on the agent's turn and ends with it handed on: the agent records what it
  did with `pass_turn` (to itself, the user, or others), as `prompts/hand-over.md` tells every run.
  A run that ends without passing hands the turn to the user with its last message. A run that
  fails is asked again by itself, in a new session, up to `failure_retries` times in a row (any
  other pass starts the count again); after that it raises an urgent item with Retry
  (`retry_urgent`: back to the agent, whose next prompt is the failed run's own, kept on its run
  record) and hands it to the user. Passing to itself keeps the clock, so after `agent_hours` the
  turn comes back to the user regardless.
- **When the agent can't run at all** (offline, `claude` missing or logged out, API overloaded,
  GitHub unreachable from `gh` or `git`), its work waits instead of failing. Each tick checks first
  that `claude` exists, `[outage] probe` and `github_probe` answer, and `claude auth status` says
  it's logged in; a failed check, or a run (on an item or a schedule) whose error says it never
  reached the agent, starts one outage: a single banner on Today with the reason and Check now
  (`check_agent`), which says what it found. Nothing is prompted, no answers are delivered and the
  queue doesn't start; no item or schedule raises its own alert. Agent's-turn items keep the turn
  (their clocks still run out to the user) and an undelivered answer stays to go again. Checks
  repeat after `first_retry_seconds`, doubling to `max_retry_seconds`, and at once when the window
  regains focus; the first good one ends the outage. One that returns within `max_retry_seconds`
  (the check passed but runs still can't get through) carries on the same wait without a new
  notification. An item on the user's turn only because its run couldn't reach the agent goes back
  to the agent once it can run.
- **The user's turn**: says what's needed (`pass_turn` requires `need`; system passes use their
  reason) and is reminded every `you_remind_hours`. Any reply (an answer, Tell Claude)
  passes the turn to the agent, unless another question on the item is open.
- **Others' turn**: after `others_hours` it goes to the item's `then`, else `others_then` (`agent`
  by default, to check whether it's still waiting and wait again or carry on; only `you` or
  `agent`). GitHub sets it from the PR: waiting on CI or reviewers is others', approved or draft is
  the user's, and failing CI, conflicts or changes requested are the agent's while Claude fixes them
  (`fix_ci`, `fix_conflicts`, `fix_reviews`), else the user's. GitHub never takes the turn from the
  agent. The agent isn't prompted on an item while an event about it waits to be handled, so the
  event's own run (a CI fix, say) is the only one.
- **Handing back**: before an agent passes anything to the user it checks it can't do it itself
  (`prompts/hand-over.md`). The user gets only what needs them: a credential or access, a judgement
  call, something away from the computer, or a merge, release or message that's theirs to make.
- **Later**: comes back when its time does, or sooner when an event in its `wake` fires (see
  Parking).

Limits live in `[turns]` (`set_turn_limits`).

CI is Quarterdeck's to watch, never an agent's. When CI finishes on an item a Claude session claimed
(`claim_item`), Quarterdeck resumes that session with the result (`prompts/ci-done.md`). A failure
nobody is on, and no `ci_failed` schedule covers, goes to a new claimed session
(`prompts/fix-ci.md`, `[github] fix_ci`).

Questions (`add_ask`) are answered, not ticked off: with one of the agent's `options` or in writing,
on Today or the item. Any open item, and every decision on Today, also takes a plain instruction
("Tell Claude what to do", `tell_claude`), delivered the same way (`prompts/message.md`). Delivery
follows on the next tick. The answer goes on the item's history, and the scheduler resumes the session
that asked (or the one holding the item) with it (`prompts/answer.md`), else starts one as on the
agent's turn. Sessions are stored with their working directory, since Claude only resumes a
session from the folder it ran in. `claim_item` and `add_ask` read the caller's session from
Claude Code (`CLAUDE_CODE_SESSION_ID`, in the MCP server's folder) and refuse one passed that has
no transcript there; one stored that Claude can no longer resume is let go, noted on the item,
and the work starts a new session.

Work Quarterdeck starts on a PR (fixing CI, acting on an answer) or on an item whose link is a
GitHub PR, issue or repo runs in that repo's project when `[[projects]]` maps it to a checkout
(`set_project`): in a git worktree per item under the data folder, in the permission mode
`project_permission_mode` sets (default `auto`, as in the user's own sessions), with the
project's `.claude/settings.local.json` copied in before every run. A PR's worktree is detached at its head; an
issue's is a new branch `issue-<n>` off the latest default branch, and the first run there also
gets the issue's text (`prompts/issue-workspace.md`); a repo's is a new branch
`quarterdeck/<item>` off it (`prompts/new-workspace.md`). The user's checkout is untouched. Since a
project's sandbox may keep Claude off GitHub, Quarterdeck does that side when a run finishes. First,
if the worktree has changes no commit holds (bar the copied settings file), the turn goes straight
back to the agent to commit them, whoever it was handed to, and nothing is published; after
`failure_retries` times in a row it's the user's, naming the worktree. Then, if no question on the
item is open, it pushes commits the branch doesn't have and, for an issue or repo,
opens a PR (closing the issue, titled from the first commit) and makes the item that PR's, in review, so syncs and
CI results follow it. A push that fails keeps the commits in the worktree: with no connection the
agent gets the turn back once it returns; otherwise the agent gets it to fix what stopped it, up
to `failure_retries` times in a row, then it's raised as urgent and the user's. Worktrees are
removed once their item is done. Repos with no project get a run that can read CI but not change
code.

`[queue]` lists items Claude works on one at a time (`set_queue`, `get_queue`). The first not yet
in review or done, and not parked or waiting before it starts, is current: when it's up next,
Quarterdeck gives the agent the turn (which starts it) and delivers `prompts/queue.md` to it like a
message; the next waits until it reaches review or done.

Default prompt files Quarterdeck wrote and nobody has edited follow newer defaults
(`.quarterdeck-defaults` records what it wrote); edited ones, and `quarterdeck.toml`, are left alone.

## Updates

Every merge to `main` builds macOS, Windows and Linux in CI and publishes a signed release. Each
installed app checks `releases/latest` every `[updates] check_minutes`, from the scheduler (no
Claude), and installs a new one only when no Claude run is live and the window isn't in use, then
restarts. So every machine runs the same build without rebuilding locally.

## Not built yet

- Slack as a native connector.
