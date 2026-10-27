{{item}}: "{{title}}" ({{link}}) now conflicts with the branch it merges into, so it can't merge.
Quarterdeck has handed it to you.

{{workspace}}

1. Fetch, then merge the base branch into the PR's branch (`gh pr view --json baseRefName` names it).
   Merge rather than rebase, so reviewers' comments stay attached.
2. If each conflict has an obvious resolution, resolve it, run what checks you can, commit and push.
3. Otherwise abort the merge, record which files conflict and why with add_note on {{item}}, and
   raise an ask with add_ask (offering options where it's a choice) for the user to decide.

Reply with one short sentence.
