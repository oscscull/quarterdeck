# Working on Quarterdeck

To install and use it, see [setup.md](setup.md).

## Build

Needs Rust, Node, and on a Mac Xcode's command line tools (`xcode-select --install`).

```bash
git clone https://github.com/oscscull/quarterdeck && cd quarterdeck && npm install
npm run tauri dev
```

The dev build runs beside an installed app without touching it: its own window ("Quarterdeck Dev", with
a banner), config (`~/.config/quarterdeck-dev/`) and database. It starts empty. To try changes on real
data, quit it and run `npm run dev:seed`, which copies the installed app's config and database in,
paused, with sync and peers off. It runs no Claude or scripts until you set `[dev] run_claude = true`
(or `run_scripts`) in its `quarterdeck.toml`.

## Check

```bash
npm run check
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test && cargo fmt --check
```

The scheduler's last error, if any, is in the database's `meta` table as `scheduler.last_error`.
[CLAUDE.md](../CLAUDE.md) has the code's rules and layout; [SPEC.md](SPEC.md) the design.

## Releases

Every merge to `main` runs `.github/workflows/release.yml`: it builds macOS (universal), Windows and
Linux and publishes release `v0.1.<run>`, which installed apps pick up. A local build is version 0.1.0,
so the next release replaces it.

Updates are signed. CI signs with two repo secrets; apps check against `plugins.updater.pubkey` and
download from `plugins.updater.endpoints`, both in `src-tauri/tauri.conf.json`. To set the key up, run
`bash scripts/release-key.sh` once (needs `gh`, Node and [dotenvx](https://dotenvx.com/docs/install)).
It makes the key in `~/.tauri/`, sets both secrets, writes the public key into `tauri.conf.json`, and
keeps the password encrypted in `.env.release`, decryptable with
`~/.tauri/quarterdeck-release.env.keys`. Losing the private key means installed apps can't update;
reinstall from a release signed with a new one.

The macOS app is code-signed with a self-signed certificate, so macOS keeps its permission answers across
updates. `bash scripts/macos-cert.sh` makes it once in `~/.tauri/quarterdeck-macos.p12` and sets the
`MACOS_CERTIFICATE` secrets. Losing it costs one more round of permission prompts.

**Forks** that publish their own releases: run both scripts, and point `plugins.updater.endpoints` at
your repo. Otherwise your builds update themselves to this repo's releases.
