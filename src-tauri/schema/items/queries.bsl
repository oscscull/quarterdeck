query items() -> ItemRow[] { list Item where (hidden = false); }
query item(id) -> ItemRow;
query item_by_external(source, external_id: text) -> ItemRow;
query items_from_source(source) -> ItemRow[];
query top_rank() -> TopRank { get Item; }
query item_count() -> ItemCount { get Item; }
query done_items(before: timestamp) -> ItemKey[] {
  list Item where (stage = done and state_since < $before);
}

# Inserts or replaces an item by id.
mutation save_item(id, title, state_label, stage: Stage, status: Status?, turn: Turn?,
                   turn_until: timestamp?, need: text?, wake: json?, waiting_on: text?,
                   then_turn: Turn?, area,
                   link: text?, source, external_id: text?, opened_at: timestamp,
                   state_since: timestamp, rank: int) -> ok {
  create Item {
    id = $id, title = $title, state_label = $state_label, stage = $stage, status = $status,
    turn = $turn, turn_until = $turn_until, need = $need, wake = $wake, waiting_on = $waiting_on,
    then_turn = $then_turn, area = $area, link = $link, source = $source, external_id = $external_id,
    opened_at = $opened_at, state_since = $state_since, rank = $rank
  } on conflict (id) update {
    title = $title, state_label = $state_label, stage = $stage, status = $status,
    turn = $turn, turn_until = $turn_until, need = $need, wake = $wake, waiting_on = $waiting_on,
    then_turn = $then_turn, area = $area, link = $link, source = $source, external_id = $external_id,
    opened_at = $opened_at, state_since = $state_since, rank = $rank
  };
}
mutation set_rank(id, rank: int) -> ok { update Item where (id = $id) { rank = $rank }; }
mutation hide_item(id) -> ok { update Item where (id = $id) { hidden = true }; }
# Asks, history and watch go with the item.
mutation delete_item(id) -> ok { delete Item where (id = $id); }

query item_events(item: Item) -> EventRow[];
mutation record_event(item: Item, at: timestamp, change: json) -> ok {
  create ItemEvent { item = $item, at = $at, change = $change };
}
