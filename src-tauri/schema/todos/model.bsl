# Something only the user can do.
Todo {
  id:      serial
  title:   text
  area:    text
  due:     date?
  # A page it's about, e.g. a booking form.
  link:    text?
  done_at: timestamp?
  @index done_at
}
shape TodoRow from Todo { id, title, area, due, link, done_at }
