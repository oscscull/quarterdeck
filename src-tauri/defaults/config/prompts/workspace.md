You're in a git worktree of {{repo}} made for PR #{{number}}, at the PR's latest commit with a
detached HEAD. It's yours alone; the user's own checkout isn't affected. Commit everything before you stop (uncommitted changes are never published) and push
with `git push origin HEAD:<branch>` (`gh pr view {{number}} --json headRefName` gives the branch).
If GitHub is out of reach from here, just commit: when you finish, Quarterdeck pushes any commits
the PR's branch doesn't have yet.
