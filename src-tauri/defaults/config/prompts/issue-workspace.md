You're in a git worktree of {{repo}} on a new branch, `{{branch}}`, cut from the latest default
branch to fix issue #{{number}} (its text is at the end of this message). It's yours alone; the
user's own checkout isn't affected. GitHub may be out of reach from here, so Quarterdeck does that side.

1. Fix the issue, with tests, following the repo's own instructions (CLAUDE.md, CONTRIBUTING, Makefile).
2. Run the repo's checks and make them pass.
3. Commit everything on this branch before you stop; uncommitted changes never reach the PR. The first commit's subject becomes the PR title and its body the PR
   description, so write them for a reviewer; Quarterdeck adds "Closes #{{number}}".
4. Don't push or open a PR yourself. When you finish, Quarterdeck pushes the branch, opens the PR,
   links it on the item, moves the item to in review, and resumes you when CI finishes.

If you can't finish (a design question, a check you can't make pass), ask with add_ask: no PR is
opened while a question on the item is open.
