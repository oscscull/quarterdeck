# CLAUDE.md

Quarterdeck is a Tauri 2 desktop app that tracks work in flight and runs recurring
Claude jobs. [docs/SPEC.md](docs/SPEC.md) describes the current design; [docs/setup.md](docs/setup.md)
how to install it and connect machines; [docs/development.md](docs/development.md) how to build and release it.

## Principles (non-negotiable)

- **Strict lints.** `cargo clippy --all-targets -- -D warnings` and `npm run check` must pass. Rust lints
  live in `src-tauri/Cargo.toml` `[lints]` and `src-tauri/clippy.toml`. Never `#[allow]` a lint without a
  `reason = "..."`; prefer fixing the code.
- **No `unwrap`, `expect` or `panic!`** outside tests. Return `AppError`.
- **Single responsibility, aggressively.** One concern per file. Short functions (clippy enforces it).
  If you want a section comment, split the file.
- **Behaviour lives in files, not code.** What Quarterdeck does (schedules, agent prompts, limits, TTLs) lives
  in the config folder (`~/.config/quarterdeck`, defaults in `src-tauri/defaults/config/`). Code implements
  *how*. To change behaviour, edit those files first.
- **All UI text lives in `src/copy/en.json`**, read through `t()`. Never hard-code copy in components.
- **Deterministic first, save tokens.** App code handles timers, polling and events. Spawn Claude only
  for judgement.
- **Nothing grows forever.** Stored agent output (runs, activity) has a TTL and is pruned.

## Documentation rules

Every file has an intended reader and a TTL (literal, or conceptual like "until the ticket merges").
Delete files that lack either. Bias towards fewer, shorter files.

- The *why* of a change goes in its commit message.
- Code comments explain non-obvious *how* only.
- `docs/SPEC.md` describes the design as it is now. Rewrite it; never append history to it.
- No decision logs, notes files or summaries.

## Commands

```bash
npm run tauri dev                                          # run a dev build beside the installed app
npm run dev:seed                                           # copy the installed app's data into it
npm run check                                              # type-check the frontend
cd src-tauri && cargo clippy --all-targets -- -D warnings  # lint Rust
cd src-tauri && cargo test                                 # Rust tests
cd src-tauri && cargo fmt --check                          # format check
```

## Layout

- `src/`: Svelte 5 frontend. `views/` one per tab, `components/` shared, `lib/` logic, `copy/` text.
- `src-tauri/src/`: Rust. `domain/` types, `store/` database access, `config/` files, `cron/`, `views/` screen
  data, `commands/` thin Tauri handlers, `claude/` headless runs, `scheduler/`, `connectors/`,
  `mcp/` (the MCP server and its tools), `peers/` (sync between machines), `alerts/`, `desktop/`.
- `src-tauri/schema/`: the database in Based (models and every query), one folder per domain.
  `src-tauri/migrations/`: generated from it with `based migrate gen`; never edit an applied one.
- `src-tauri/defaults/config/`: default config files, embedded in the binary.

Every way to change data should exist as an MCP tool, since agents are the main way in.
