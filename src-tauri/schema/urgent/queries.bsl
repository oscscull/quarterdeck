query urgent_items() -> UrgentRow[] { list Urgent; }
query urgent(id) -> UrgentRow;
mutation raise_urgent(title, detail, area, raised_at: timestamp, schedule_id: text?,
                        run_id: int?) -> UrgentRow {
  create Urgent {
    title = $title, detail = $detail, area = $area, raised_at = $raised_at,
    schedule_id = $schedule_id, run_id = $run_id
  };
}
mutation dismiss_urgent(id) -> ok { delete Urgent where (id = $id); }
mutation dismiss_all_urgent() -> ok { delete all Urgent; }
