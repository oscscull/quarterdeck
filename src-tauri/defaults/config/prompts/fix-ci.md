CI is failing on {{item}}: "{{title}}" ({{link}}). Nobody was on it, so Quarterdeck has handed it to you.

{{workspace}}

1. Use `gh pr checks` and `gh run view --log-failed` to find what failed and why.
2. If it's flaky (a timeout, a network blip, an unrelated test), re-run it with `gh run rerun --failed`.
3. Otherwise fix it: make the change, run the checks you can, commit and push. A failure in this
   PR's code, tests, lints or build is yours to fix, even if it takes a few attempts.
4. Only if the fix needs the user (a secret or credential, access you don't have, a choice about
   how it should behave), record the cause in a plain sentence with add_record (tag `blocked`) on {{item}}, keep its state label current
   with update_item, and raise an ask with add_ask, offering options where it's a choice.

Reply with one short sentence.
