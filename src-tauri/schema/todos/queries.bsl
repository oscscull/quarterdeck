# Open to-dos (soonest due first, undated last), then done ones, latest first.
query todos() -> TodoRow[] {
  list Todo order (done_at desc nulls first, due asc nulls last, id);
}
query todo(id) -> TodoRow;
mutation add_todo(title, area, due: date?, link: text?, done_at: timestamp?) -> ok {
  create Todo { title = $title, area = $area, due = $due, link = $link, done_at = $done_at };
}
mutation update_todo(id, title, area, due: date?, link: text?, done_at: timestamp?) -> ok {
  update Todo where (id = $id) {
    title = $title, area = $area, due = $due, link = $link, done_at = $done_at
  };
}
mutation set_todo_done(id, done_at: timestamp?) -> ok {
  update Todo where (id = $id) { done_at = $done_at };
}
mutation delete_todo(id) -> ok { delete Todo where (id = $id); }
mutation expire_todos(before: timestamp) -> ok {
  delete Todo where (done_at != null and done_at < $before);
}
