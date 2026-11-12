shape ProjectRow from Project {
  id, title, aim, area, state, rank, opened_at, state_since, briefing, briefed_at
}
shape TopProjectRank from Project { top = max(rank) }

query projects() -> ProjectRow[];
query project(id) -> ProjectRow;
query projects_in(area) -> ProjectRow[] { list Project where (area = $area); }
query top_project_rank() -> TopProjectRank { get Project; }

# Inserts or replaces a project by id, leaving its briefing alone.
mutation save_project(id, title, aim, area, state: ProjectState, rank: int,
                      opened_at: timestamp, state_since: timestamp) -> ok {
  create Project {
    id = $id, title = $title, aim = $aim, area = $area, state = $state, rank = $rank,
    opened_at = $opened_at, state_since = $state_since
  } on conflict (id) update {
    title = $title, aim = $aim, area = $area, state = $state, rank = $rank,
    state_since = $state_since
  };
}
mutation set_project_briefing(id, briefing: json, briefed_at: timestamp) -> ok {
  update Project where (id = $id) { briefing = $briefing, briefed_at = $briefed_at };
}
# Its references go with it; its items stay, ungrouped.
mutation delete_project(id) -> ok { delete Project where (id = $id); }

shape AreaBriefingRow from AreaBriefing { area, text, written_at }
query area_briefing(area) -> AreaBriefingRow;
mutation save_area_briefing(area, text: json, written_at: timestamp) -> ok {
  create AreaBriefing { area = $area, text = $text, written_at = $written_at }
    on conflict (area) update { text = $text, written_at = $written_at };
}
