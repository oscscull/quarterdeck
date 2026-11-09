# Setting up Quarterdeck

To work on Quarterdeck's code instead, see [development.md](development.md).

## Install

1. Have [`gh`](https://cli.github.com) logged in (`gh auth login`) and
   [Claude Code](https://claude.com/claude-code) (`claude`) logged in.
2. Download this machine's build from the
   [latest release](https://github.com/oscscull/quarterdeck/releases/latest). macOS: unpack the
   `.app.tar.gz` into `/Applications`, then, since it isn't notarized, run once:
   `xattr -dr com.apple.quarantine /Applications/Quarterdeck.app`. It updates itself after that.
3. Open it. The first start creates the config folder `~/.config/quarterdeck/` and the database
   (macOS `~/Library/Application Support/com.oscscull.quarterdeck/`, Windows
   `%APPDATA%\com.oscscull.quarterdeck\`, Linux `~/.local/share/com.oscscull.quarterdeck/`).
4. Connect Claude Code: run the command under "Connect your Claude sessions" on the Agent tab. From
   then on, Claude can set up the rest through Quarterdeck's tools ("map my-org to work", "add my
   calendar: <link>").

## Configure

Everything lives in `~/.config/quarterdeck/` and applies as soon as it's saved. Its `README.md` explains
the folder; `quarterdeck.toml` explains each setting in its comments. The first edits to make:

| Setting | What to do |
| --- | --- |
| `node` | Name this machine. |
| `[[areas]]` | Your areas (default Work, Projects, Life). Set each area's `node` to the machine that runs its jobs; with one machine, that's all of them. |
| `[github.areas]` | Map GitHub owners or repos to areas. Unmapped ones go to `[github] default_area`. |
| `[[projects]]` | Where repos are checked out. Without one, Claude can't work on that repo's PRs or issues. |
| `[[calendar.feeds]]` | A private iCal `url`, or on a Mac a `mac_calendar` title from the Calendar app (for accounts with no iCal link, such as many work Google accounts: add the account in System Settings → Internet Accounts, and allow access when macOS asks). |

Optional, all with working defaults:

| Section | Controls |
| --- | --- |
| top level | Launch at login, Priorities count, chat history, the `claude` command, how Claude may act in worktrees, how long stored output lives. |
| `[github]` | Polling, ignored owners/repos, self-review repos, auto-fixing CI, updating behind branches, fixing conflicts. |
| `[github.issues]` | Owners or repos whose issues become items. |
| `[github.informational_checks]` | CI checks whose failures only inform. |
| `[queue]` | Items Claude works on one at a time, in order. |
| `[calendar]`, `[alerts]` | Calendar polling and filters; the countdown before meetings. |
| `[turns]` | How long each party (you, Claude, others) holds an item before it moves on. |
| `[outage]` | Retrying when Claude or GitHub can't be reached. |
| `[sync]` | Sharing with other machines (below). |
| `[updates]` | Auto-update. |

The rest of the folder:

- `schedules/`: recurring jobs, one file each. Easiest made from the Agent tab or by asking Claude.
- `prompts/`: what Claude is told for each built-in job. Edit one to change its behaviour.
- `inbox/`, `pause.toml`: written by agents and the app; see the folder's `README.md`.

Updates bring new prompts and settings but don't rewrite your `quarterdeck.toml` or a prompt you've
edited. To pick up a new setting, copy it from
[the defaults](../src-tauri/defaults/config/quarterdeck.toml).

## Two machines

Each machine keeps its own data and serves it to the other. Jobs and connectors run only for areas
whose `node` is this machine.

1. Give each machine its own `node`, and the same `[[areas]]` on both. They keep the list in step after
   that; a clash is raised as urgent.
2. On each, set `[sync] enabled = true` (or ask Claude: `set_sync`) and restart. A token appears in
   `[sync] token`.
3. Make each reachable:
   - **Same network:** `[sync] lan = true`; the URL is `http://<name>.local:4791`, where `<name>` is
     `scutil --get LocalHostName`. Use the name, not an IP: it survives VPNs that claim your home
     network's range.
   - **Across networks:** a tunnel. Tailscale: `tailscale serve --bg --tcp 4791 tcp://localhost:4791`,
     URL `http://<machine>.<tailnet>.ts.net:4791`. Or SSH: `ssh -N -L 4792:localhost:4791 other-mac`,
     URL `http://localhost:4792`.
4. On each, add the other as a peer: ask Claude (`add_peer` with its `node`, URL and token), or add a
   `[[sync.peers]]` entry.
5. The Agent tab's Connections list shows whether each peer answers.

A machine without a screen can run `<binary> serve` instead of the app.

## When something's wrong

- macOS says the app is damaged: rerun the `xattr` command from Install.
- No PRs appear: check `gh auth status`.
- Claude never runs: check `claude` works in a terminal, or set `claude_command` to its full path.
- Ask Claude to call `get_overview` and `get_run_log`; it can usually tell you what's stuck.
