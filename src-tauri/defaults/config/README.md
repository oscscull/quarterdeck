# Quarterdeck config

Read by: you, and agents asked to change what Quarterdeck does. Quarterdeck re-reads this folder on
every refresh, so edits apply immediately.

- `quarterdeck.toml`: mechanical settings, how long stored output lives, and your areas (e.g. Work,
  Projects, Life), each with the machine that runs its schedules.
- `[queue]` in `quarterdeck.toml`: items Claude works on one at a time, in order.
- `prompts/`: Quarterdeck's built-in agent jobs. `{{name}}` placeholders are filled in by the app.
- `schedules/`: one file per recurring job; the file name is its id. TOML between `+++`
  lines, then the instructions Claude follows. Copy an existing file to start a new one.
  `allowed_tools` (optional) lists Claude Code tools a run may use without asking, e.g.
  `["Bash(gh pr comment:*)"]`; without it a run can only read and report back.
  Each `[[triggers]]` entry starts it: `type = "cron"` with `expr`, or `type = "event"` with
  `on` (an event name: `github.ci_failed`, `item.turn`, `run.finished`…, or `custom.<anything>`),
  optional `when` (fields the event must have, `*` as a wildcard, e.g. `when = { to = "agent" }`)
  and optional `cooldown` (minutes between runs on the same subject). No triggers means it only
  runs when run by hand. Events are detected by app code and cost no tokens until they fire.
  A `[script]` table (`cwd`, `timeout_minutes`, `report` = `failures`/`always`/`never`) makes the
  body a shell script instead of instructions for Claude. Scripts written outside the app's editor
  wait for you to approve them on the Agent tab.
- `inbox/`: an agent raises an urgent item by writing `inbox/<anything>.toml` with `title` and
  `detail` (and optionally `schedule_id`). Quarterdeck picks it up and deletes the file.
- `pause.toml`: present while every schedule is paused; `until` is when it ends (absent means
  until resumed). Delete it to resume.
