# Runs that have started but not finished.
query live_runs() -> RunRow[] { list Run where (finished_at = null) order (started_at, id); }
# The latest finished runs of one schedule.
query recent_runs(schedule_id: text, limit: int) -> RunRow[] {
  list Run where (schedule_id = $schedule_id and finished_at != null)
    order (started_at desc, id desc) limit ($limit, max 100);
}
mutation add_run(schedule_id: text?, what, target, node, started_at: timestamp,
                 finished_at: timestamp?, outcome: Outcome?, summary: text?) -> RunRow {
  create Run {
    schedule_id = $schedule_id, what = $what, target = $target, node = $node,
    started_at = $started_at, finished_at = $finished_at, outcome = $outcome,
    summary = $summary
  };
}
mutation finish_run(id, at: timestamp, outcome: Outcome, summary: text) -> ok {
  update Run where (id = $id) { finished_at = $at, outcome = $outcome, summary = $summary };
}
# Runs still marked as going when the app starts can't be: their process ended with the app.
mutation close_interrupted_runs(at: timestamp) -> ok {
  update Run where (finished_at = null) {
    finished_at = $at, outcome = failed, summary = "stopped when Quarterdeck quit"
  };
}
mutation forget_runs(schedule_id: text) -> ok { delete Run where (schedule_id = $schedule_id); }
mutation expire_runs(before: timestamp) -> ok {
  delete Run where (finished_at != null and finished_at < $before);
}

query run_requests() -> RunRequestRow[] { list RunRequest; }
mutation request_run(schedule_id) -> ok { create RunRequest { schedule_id = $schedule_id }; }
mutation clear_run_requests() -> ok { delete all RunRequest; }
# The latest finished runs about one item.
query item_runs(target: text, limit: int) -> RunRow[] {
  list Run where (target = $target and finished_at != null)
    order (finished_at desc, id desc) limit ($limit, max 10);
}
# What a run was asked, to ask it again.
query run_retry(id) -> RunRetryRow;
mutation set_run_prompt(id, prompt: text) -> ok { update Run where (id = $id) { prompt = $prompt }; }
