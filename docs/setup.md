# Setting up Quarterdeck

## One machine

1. Needs: `gh auth login` done, Claude Code (`claude`) logged in.
2. Download the app for this machine from the
   [latest release](https://github.com/oscscull/quarterdeck/releases/latest) (macOS: the `.app.tar.gz`,
   unpacked into `/Applications`). The build isn't notarized, so the first time on a Mac, clear the
   download flag: `xattr -dr com.apple.quarantine /Applications/Quarterdeck.app`. After that it updates
   itself from each new release (`[updates]` in `quarterdeck.toml`).
3. To work on Quarterdeck instead: needs Rust and Node too, and on a Mac Xcode's command line tools
   (`xcode-select --install`); `git clone https://github.com/oscscull/quarterdeck
   && cd quarterdeck && npm install`, then `npm run tauri dev`. A local build is version 0.1.0, so the
   next release replaces it.
4. First start creates `~/.config/quarterdeck/` (settings, prompts, schedules, inbox) and the database
   (`quarterdeck.db`) in the system's app-data folder (macOS
   `~/Library/Application Support/com.oscscull.quarterdeck/`, Windows `%APPDATA%\com.oscscull.quarterdeck\`,
   Linux `~/.local/share/com.oscscull.quarterdeck/`).
5. In `~/.config/quarterdeck/quarterdeck.toml`, set `node` to this machine's name, check the `[[areas]]`
   (each area's `node` is the machine that runs it), map GitHub owners to areas under
   `[github.areas]`, and add calendars under `[[calendar.feeds]]`: a private iCal link (`url`), or
   a calendar's title in the macOS Calendar app (`mac_calendar`). The Calendar app suits accounts
   that hide iCal links, such as many work Google accounts: add the account in System Settings →
   Internet Accounts, and allow Quarterdeck when macOS asks for calendar access.
6. Connect Claude Code: run the command shown under "Connect your Claude sessions" on the Agent
   tab. After that, most setup can be done by asking Claude ("add my work calendar: <link>").

## Two machines

Each machine keeps its own data and serves it to the other. Records stay where they were made;
connectors and schedules only run for areas whose `node` is this machine.

1. Give each machine a different `node`, and the same `[[areas]]` list on both. After that the
   machines keep the list in step themselves; if both change it before they next talk, Quarterdeck
   raises it as urgent and you make one match the other.
2. On each machine, turn sharing on: `[sync] enabled = true` (or Claude: `set_sync`), then restart
   Quarterdeck. A token is written to `[sync] token`; `get_sync_info` shows it.
3. Make each machine reachable. On the same network (simplest): `[sync] lan = true` (or Claude:
   `set_sync` with `lan`), and use `http://<LocalHostName>.local:4791`, where `<LocalHostName>` is
   `scutil --get LocalHostName` on that Mac. Only private addresses may connect, and macOS may ask
   once to allow incoming connections. Use the `.local` name, not an IP: if a company VPN claims your
   home network's IPv4 range (common when it's `10.x`), the name still reaches the other Mac over
   IPv6 link-local (`fe80::`), which VPNs leave on the local network. Across networks, expose the port with a tunnel. With Tailscale, enable
   Serve for the tailnet once (the command prints the link), then on each machine run
   `tailscale serve --bg --tcp 4791 tcp://localhost:4791` and use
   `http://<machine>.<tailnet>.ts.net:4791`. (The HTTPS form, `tailscale serve --bg 4791`, also
   works but needs Tailscale's certificate service reachable; it fails on some networks.) Or use
   `ssh -N -L 4792:localhost:4791 other-mac` (then `http://localhost:4792`).
4. On each machine, add the other as a peer: Claude `add_peer` with its `node` name, URL and token,
   or a `[[sync.peers]]` entry in `quarterdeck.toml`.
5. The Agent tab's Connections list shows whether each peer answers.

A machine without a screen can serve its Quarterdeck with `<binary> serve`.

## Developing

`npm run tauri dev` runs a dev build beside the installed app, never touching its data: its own
window ("Quarterdeck Dev", with a banner on every screen), config (`~/.config/quarterdeck-dev/`) and
database. It starts empty. To work on your real data, quit the dev app and run `npm run dev:seed`,
which copies the installed app's config and database in, paused, with sync and peers off. A dev build
runs no Claude or scripts until you set `[dev] run_claude = true` (or `run_scripts`) in its
`quarterdeck.toml`.

## Checking it

- Lints and tests: `npm run check`, then in `src-tauri/`: `cargo clippy --all-targets -- -D warnings`
  and `cargo test`.
- The scheduler's last error, if any, is in the database's `meta` table as `scheduler.last_error`.

## Releases

Every merge to `main` runs `.github/workflows/release.yml`: it builds macOS (universal), Windows and
Linux, and publishes release `v0.1.<run>`, which installed apps pick up. Updates are signed; the
public key is `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`, and CI signs with two repo
secrets. To set them up, the repo owner runs `bash scripts/release-key.sh` once (needs `gh`, Node and
[dotenvx](https://dotenvx.com/docs/install)). It asks for a password, makes the signing key in
`~/.tauri/`, sets both secrets, writes the public key into `tauri.conf.json`, and keeps the password
only encrypted in `.env.release`; what decrypts it stays in `~/.tauri/quarterdeck-release.env.keys`.

Losing the private key means installed apps can't update; reinstall from a release signed with a new one.

The macOS app is also code-signed with a self-signed certificate, so macOS keeps its permission answers
(Documents, Calendars) across updates instead of asking again after each one. `bash scripts/macos-cert.sh`
makes it once in `~/.tauri/quarterdeck-macos.p12` and sets the `MACOS_CERTIFICATE` secrets; its password
sits encrypted in `.env.release` beside the updater's. Losing it costs one more round of prompts after
the next release.

