# How a project is going, as the Briefing says it.
enum ProjectState { moving, stalled, quiet, done }

# Work towards one aim, grouping items within an area, e.g. "Writer" at work.
@sort(rank)
@key(id)
Project {
  id:          text
  title:       text
  # The aim, in a sentence: what the work is for.
  aim:         text
  area:        text
  state:       ProjectState
  rank:        int
  opened_at:   timestamp
  state_since: timestamp
  # The project's own briefing (`Briefing` as JSON) and when it was written.
  briefing:    json?
  briefed_at:  timestamp?
  @index(area, rank)
}

# One area's combined briefing, across its projects.
@key(area)
AreaBriefing {
  area:       text
  text:       json
  written_at: timestamp
}
