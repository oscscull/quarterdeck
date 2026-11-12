mutation fire_event(kind, area, subject: text, data: json, about: text, depth: int,
                    at: timestamp) -> ok {
  create FiredEvent {
    kind = $kind, area = $area, subject = $subject, data = $data, about = $about,
    depth = $depth, at = $at
  };
}
# Events the scheduler hasn't looked at yet, oldest first.
query new_events(limit: int) -> FiredRow[] {
  list FiredEvent where (handled = false) order (id) limit ($limit, max 500);
}
mutation handle_event(id, started: json) -> ok {
  update FiredEvent where (id = $id) { handled = true, started = $started };
}
# The latest events, for the Agent tab.
query recent_events(limit: int) -> FiredRow[] {
  list FiredEvent where (handled = true) order (id desc) limit ($limit, max 100);
}
mutation expire_events(before: timestamp) -> ok { delete FiredEvent where (at < $before); }
