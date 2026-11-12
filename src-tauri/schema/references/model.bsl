enum RefKind { you_said, ticket, pr, comment, slack, doc, commit, config, other }
enum RefBy   { you, agent }

# A source the briefing cites: something said, a ticket, a thread, a doc. `quote` is the data
# point that matters.
@sort(at desc, id desc)
Reference {
  id:      serial
  area:    text
  kind:    RefKind
  title:   text
  url:     text?
  quote:   text
  by:      RefBy
  at:      timestamp
  project: Project? @fk("a project's references go with it", on_delete: cascade)
  @index(area, at)
}

# Which items a reference is about.
@sort(reference)
@key(reference, item)
ReferenceItem {
  reference: Reference @fk("links go with their reference", on_delete: cascade)
  item:      Item @fk("links go with their item", on_delete: cascade)
}
