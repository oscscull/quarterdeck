shape RefRow from Reference { id, area, kind, title, url, quote, by, at, project { id } }
shape RefLink from ReferenceItem { reference_id = reference.id, item_id = item.id }

query references_in(area) -> RefRow[] { list Reference where (area = $area); }
query reference(id) -> RefRow;
query reference_links() -> RefLink[];

mutation add_reference(area, kind: RefKind, title, url: text?, quote, by: RefBy, at: timestamp,
                       project: Project?) -> RefRow {
  create Reference {
    area = $area, kind = $kind, title = $title, url = $url, quote = $quote, by = $by, at = $at,
    project = $project
  };
}
mutation update_reference(id, kind: RefKind, title, url: text?, quote, project: Project?) -> ok {
  update Reference where (id = $id) {
    kind = $kind, title = $title, url = $url, quote = $quote, project = $project
  };
}
mutation remove_reference(id) -> ok { delete Reference where (id = $id); }

mutation link_reference(reference: Reference, item: Item) -> ok {
  create ReferenceItem { reference = $reference, item = $item };
}
mutation clear_reference_links(reference: Reference) -> ok {
  delete ReferenceItem where (reference = $reference);
}
