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

- **Today**: next meeting, Priorities (in rank order: questions on an item, else the item itself
  when it's in flight and the user's turn), To do (things only the user can do), and "From Claude".
- **Priorities**: everything in play, drag to rank (per machine). The ranking drives Today.
- **Board**: items by stage. **In flight**: open items by whose turn it is.
- **Agent**: every item that is the agent's turn, and what's running (each run's latest step, live; click for its steps so far), every schedule (toggle, edit, delete, run now, create by describing
  it), missed or failed runs, Pause all, connector and peer status, and the MCP connect command.
- **Ask Claude** bubble on every view, given that view's data and the Quarterdeck tools.

## Where things live

- **Config folder** `~/.config/quarterdeck/`, editable by people and agents, re-read on each refresh:
  `quarterdeck.toml` (settings, areas, connectors, sync), `prompts/*.md` (Quarterdeck's own Claude jobs),
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
  every screen, no updater, and no Claude runs unless their `[dev] run_claude` is on.
  `npm run dev:seed` replaces a dev build's config and data with a snapshot of the installed app's,
  paused and with sync and peers off.

## Connectors

Populate by default, never fight overrides. Each keeps the last snapshot it saw and only re-applies
the fields it owns when they change upstream; hidden or re-filed records stay that way.

- **GitHub** (`gh` CLI): open PRs become items; new comments, CI failing and CI passing fire events.
  CI state is worked out per check: checks a repo marks informational (`informational_checks`, e.g.
  a visual-diff report) don't count, and their results are noted on the item instead.
  PRs with no requested reviewers, or in `self_review` repos, wait on the user. An item is closed
  only when `gh` says its PR merged or closed, never because the open-PR search missed it. Syncs
  every `poll_minutes`, and at once when the window regains focus (at most every
  `focus_sync_seconds`), after which the screen reloads.
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
stops everything. Event triggers cost nothing until they fire.

## Turns

Every item has a turn, `you`, `agent`, `others` (with who: reviewers, CI) or `later`, and every
item in flight has a clock on it, so nothing idles. Turns change only through `turns::pass`, which
records why on the item's history; the UI and peers read the stored turn and never derive it.

- **The agent's turn**: Quarterdeck prompts it. Each tick, an agent's-turn item with nothing
  running gets a run (`prompts/continue.md`), resuming the session holding it or starting one; a
  session working outside Quarterdeck is left alone until it goes quiet for `quiet_minutes`. Every
  run on an item starts on the agent's turn and ends with it handed on: the agent records what it
  did with `pass_turn` (to itself, the user, or others), as `prompts/hand-over.md` tells every run.
  A run that ends without passing hands the turn to the user with its last message; a run that
  fails raises an urgent item and hands it to the user. Passing to itself keeps the clock, so after
  `agent_hours` the turn comes back to the user regardless.
- **The user's turn**: reminded every `you_remind_hours`. Any reply (an answer, Tell Claude)
  passes the turn to the agent, unless another question on the item is open; one that no session
  or worktree can take is raised as urgent and the turn comes back.
- **Others' turn**: after `others_hours` it goes to the item's `then`, else `others_then` (`you`
  by default). GitHub sets it from the PR: waiting on CI or reviewers is others', approved, draft or
  changes requested is the user's. GitHub never takes the turn from the agent.
- **Later**: comes back when its time does.

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
GitHub issue runs in that repo's project when `[[projects]]` maps it to a checkout
(`set_project`): in a git worktree per item under the data folder, in the permission mode
`project_permission_mode` sets (default `auto`, as in the user's own sessions), with the
project's `.claude/settings.local.json` copied in before every run. A PR's worktree is detached at its head; an
issue's is a new branch `issue-<n>` off the latest default branch, and the first run there also
gets the issue's text (`prompts/issue-workspace.md`). The user's checkout is untouched. Since a
project's sandbox may keep Claude off GitHub, Quarterdeck does that side when a run finishes and no
question on the item is open: it pushes commits the branch doesn't have and, for an issue, opens a
PR closing it (titled from the first commit) and makes the item that PR's, in review, so syncs and
CI results follow it. A push that fails is raised as urgent and the turn goes to the user. After every
run, finished or failed, the project's `clean_after_run` folders (build output, relative to the repo
root) are deleted from the worktree, so worktrees waiting on review hold no build output. Worktrees are
removed once their item is done. Repos with no project get a run that can read CI but not change
code.

`[queue]` lists items Claude works on one at a time (`set_queue`, `get_queue`). The first not yet
in review or done is current: when it's up next, Quarterdeck marks it in progress, gives the agent
the turn and delivers `prompts/queue.md` to it like a message; the next waits until it reaches
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
