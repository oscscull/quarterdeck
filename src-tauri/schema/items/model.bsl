enum Stage  { up_next, in_progress, in_review, done }
# `proceeding` is retired and unread. Removing it makes Based rebuild the table, which breaks
# other tables' foreign keys on SQLite.
enum Status { decision, review, ci, ci_failed, dependency, approved, proceeding, queued }
# Whose move it is on an item. Every open item (up next, in progress, in review) has one; done
# items have none. Open rows stored without one read as the user's turn.
enum Turn   { you, agent, others, later }

# A piece of work. `id` is a ticket key (JD-104), a PR (app#12) or a made-up one (B-3).
@sort(rank)
@key(id)
Item {
  id:          text
  title:       text
  state_label: text
  stage:       Stage
  status:      Status?
  turn:        Turn?
  # When the turn runs out.
  turn_until:  timestamp?
  # On the user's turn: what's needed from them, in a line.
  need:        text?
  # What wakes a later turn early: event triggers (`OnEvent` as JSON), in the scheduler's format.
  wake:        json?
  # Who the item waits on while it's others' turn, e.g. reviewers' names or "CI".
  waiting_on:  text?
  # Who gets the turn when others' turn runs out (none: the configured others_then) or when a
  # later turn's time comes (none: you).
  then_turn:   Turn?
  area:        text
  link:        text?
  # "manual" for items made by hand or by an agent, else the connector that made it.
  source:      text (default "manual")
  external_id: text?
  # Synced items the user deleted stay hidden, so the next sync doesn't bring them back.
  hidden:      bool (default false)
  opened_at:   timestamp
  state_since: timestamp
  rank:        int
  # The project it belongs to, if any. Deleting the project ungroups its items.
  project:     Project? @fk("items outlive their project", on_delete: set_null)
  @index(source, external_id) unique
  @index(hidden, rank)
  @index(stage, state_since)
}

shape ItemRow from Item {
  id, title, state_label, stage, status, turn, turn_until, need, wake, waiting_on, then_turn,
  area, link, source, external_id, opened_at, state_since, rank
}
shape ItemProject from Item { id, project_id = project.id }
shape ItemKey from Item { id }
shape TopRank from Item { top = max(rank) }
shape ItemCount from Item { n = count() }

# One line of an item's history. `change` is the app's `Change` enum as JSON.
@sort(at desc, id desc)
ItemEvent {
  id:     serial
  item:   Item @fk("history goes with its item", on_delete: cascade)
  at:     timestamp
  change: json
  # Machine bookkeeping (stage, turn, label...) rather than a record written for people. Pruned
  # after `trace_ttl_days`.
  trace:  bool (default false)
  @index(item, at)
  @index(trace, at)
}
shape EventRow from ItemEvent { id, item_id = item.id, at, change, trace }
