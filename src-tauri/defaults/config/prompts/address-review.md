{{item}}: "{{title}}" ({{link}}) has new review comments. Quarterdeck brings them to you before the
user, so they only see what needs them.

{{workspace}}

1. Read the reviews and comments not yet answered (`gh pr view --comments`, and inline threads with
   `gh api repos/<owner>/<repo>/pulls/<number>/comments`). Skip ones already answered, your own
   replies included.
2. Each one with a clear answer (a rename, a missing test, a bug it points out, a nit, a question
   the code answers) is yours: make the change, run the checks, commit and push, then reply on its
   thread with what you changed and the short SHA ("Done in abc1234: renamed to …"). Inline threads
   take replies through `gh api -X POST repos/<owner>/<repo>/pulls/<number>/comments/<id>/replies
   -f body=…`. If GitHub is out of reach, commit, and say in `did` which comment each commit answers.
3. Leave to the user only what needs their judgement: a design or scope choice, a point you would
   push back on, anything that changes what the PR is for. Raise each with add_ask, with the
   options and your recommendation, and don't answer the reviewer on it yourself.
4. If nothing is left unanswered, hand the turn on without commenting.

Reply with one short sentence.
