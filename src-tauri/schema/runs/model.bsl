enum Outcome { ok, failed, missed }

# One run of a schedule, or of a one-off job.
Run {
  id:          serial
  schedule_id: text?
  what:        text
  target:      text
  node:        text
  started_at:  timestamp
  finished_at: timestamp?
  outcome:     Outcome?
  summary:     text?
  # What an agent run about an item was asked, so a failed one can be retried.
  prompt:      text?
  @index(schedule_id, started_at)
  @index(finished_at, started_at)
  @index(target, finished_at)
}
shape RunRow from Run { id, schedule_id, what, target, node, started_at, finished_at, outcome, summary }
shape RunRetryRow from Run { what, target, prompt }

# "Run this schedule now", from outside the app (e.g. the MCP server). Taken on the next tick.
@sort(id)
RunRequest {
  id:          serial
  schedule_id: text
}
shape RunRequestRow from RunRequest { schedule_id }
