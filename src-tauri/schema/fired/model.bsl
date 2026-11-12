# Something that happened, which starts the schedules whose triggers match it. Written by
# connectors, item changes, runs and agents; the scheduler takes new ones each tick.
@sort(id)
FiredEvent {
  id:      serial
  kind:    text
  area:    text
  # What it's about: an item or schedule id, or empty.
  subject: text
  # Its fields as a JSON object of strings, for triggers' `when` filters.
  data:    json
  # One line for the run, e.g. "CI failing on me/app#7 (https://…)".
  about:   text
  # How many events led to this one, so schedules can't start each other forever.
  depth:   int
  at:      timestamp
  handled: bool (default false)
  # Ids of the schedules it started, as a JSON array.
  started: json?
  @index(handled, id)
  @index(at)
}
shape FiredRow from FiredEvent { id, kind, area, subject, data, about, depth, at, started }
