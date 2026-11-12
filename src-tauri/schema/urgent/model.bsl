# Something that needs the user now. Dismissing deletes it.
@sort(raised_at desc, id desc)
Urgent {
  id:          serial
  title:       text
  detail:      text
  area:        text
  raised_at:   timestamp
  schedule_id: text?
  # Set when it is about a failed agent run, which lets the UI offer "Retry".
  run_id:      int?
}
shape UrgentRow from Urgent { id, title, detail, area, raised_at, schedule_id, run_id }
