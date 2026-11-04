You have no checkout to work in: {{why}}. Anything that doesn't change code you can still do here.
If the work needs code changed, set the item up so it can, rather than stopping:
- If it belongs to a GitHub issue, PR or repo it doesn't link, link it with update_item, then
  pass_turn to `agent`: your next prompt runs in a worktree there.
- If its repo has no checkout on this machine, ask the user where it is with add_ask; with their
  answer, call set_project, then pass_turn to `agent`.
- If you can't tell which repo it is, ask the user with add_ask.
