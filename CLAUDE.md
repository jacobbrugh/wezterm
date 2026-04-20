# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

WezTerm — a GPU-accelerated cross-platform terminal emulator and multiplexer written in Rust. User-facing docs live at <https://wezterm.org/>.

    This is a public fork maintained by jacobbrugh. DO NOT publish any
    user-specific information (e.g. hostnames, local paths) and ESPECIALLY avoid
    pushing any sensitive information

## Common commands

Build / check / test are driven through `make` and `cargo`. Minimum Rust version is **1.71** (see `ci/check-rust-version.sh`).

```
make build         # cargo build -p {wezterm,wezterm-gui,wezterm-mux-server,strip-ansi-escapes}
make check         # cargo check across the main crates incl. wezterm-escape-parser/-cell/-surface/-ssh
make test          # cargo nextest run, then nextest for wezterm-escape-parser (no_std default features)
make fmt           # cargo +nightly fmt  -- nightly is required because of rustfmt options in use
make docs          # builds the mkdocs site via podman/docker (ci/build-docs.sh)
make servedocs     # serves docs with live reload on :8000
```

Fast-iteration workflow (matches `CONTRIBUTING.md`): use `cargo check` to type-check, `cargo run` for a debug build, `RUST_BACKTRACE=1 cargo run` for backtraces. Tests use **cargo-nextest**, not `cargo test`. To run a single test: `cargo nextest run -p <crate> <test_name_substring>`.

`wezterm-escape-parser` is `no_std` by default, so check/test it separately (as the Makefile does) when touching escape-sequence parsing.

CI workflows under `.github/workflows/gen_*` are **generated** by `ci/generate-workflows.py`; edit the generator, then run `ci/update-derived-files.sh` — do not hand-edit the `gen_*.yml` files.

## Architecture

WezTerm is a Cargo workspace (~60 crates). The layering below is the big picture; most work happens in one of four places.

### Core terminal model — `term/` (`wezterm-term` crate)

The headless terminal emulator: escape-sequence parsing, screen + scrollback model, keyboard/mouse input encoding, sixel/iTerm2/kitty image protocols, OSC 8 hyperlinks. It is GUI-agnostic — callers feed bytes in via `Terminal::advance_bytes` and receive a `std::io::Write` for PTY output. The `TerminalState` (under `term/src/terminalstate/`) is split by concern: `performer.rs` (VT parser callback), `keyboard.rs`, `mouse.rs`, `image.rs`, `iterm.rs`, `kitty.rs`, `sixel.rs`. Add new escape sequences here; aim for xterm compatibility (<https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>).

### Terminal primitives — `termwiz/`

General-purpose Rust crate (published to crates.io) for terminal applications: `Surface`/`Cell` model, `Change` deltas, escape parser + re-encoder, capability probing, widgets, line editor, Unix-tty + Windows-console abstractions. Used by `term`, the mux, and standalone utilities. Has its own `CHANGELOG.md` and release cadence.

### Multiplexer — `mux/`

Owns the collection of windows/tabs/panes, domains (local, SSH, TLS, tmux-control-mode, unix-socket), clipboard routing, and the pane renderable view consumed by the GUI. A `Domain` creates `Pane`s; `Tab`s hold panes in a split layout; `Window`s hold tabs. Entry points live in `mux/src/lib.rs` (`Mux`), `domain.rs`, `localpane.rs`, `tab.rs`, `window.rs`. The mux can run standalone as `wezterm-mux-server` (headless) or embedded inside the GUI.

### GUI — `wezterm-gui/`

The `wezterm-gui` binary. Renders via glium/wgpu (`renderstate.rs`, `shader.wgsl`, `glyph-*.glsl`), manages the OS window through the in-tree `window/` crate, and composes the terminal UI in `src/termwindow/` (`mod.rs`, `render/`, `keyevent.rs`, `mouseevent.rs`, `selection.rs`, `resize.rs`, `tabbar.rs`, `palette.rs`, etc.). Lua-scriptable behavior is wired up under `src/scripting/`.

### CLI — `wezterm/`

The `wezterm` command (CLI wrapper, subcommands for the multiplexer, `imgcat`, `cli ...`, etc.). Subcommands live in `wezterm/src/cli/` (each file = one subcommand).

### Config + Lua — `config/` and `lua-api-crates/`

`config/` defines the Rust-side config schema, with `config-derive/` for proc macros and `wezterm-dynamic/` providing a serde-like dynamic value type used to bridge Rust and Lua.

`lua-api-crates/` is split into many small crates (`battery`, `color-funcs`, `filesystem`, `mux`, `plugin`, `spawn-funcs`, `ssh-funcs`, `termwiz-funcs`, `time-funcs`, `url-funcs`, `window-funcs`, ...) purely to parallelize compilation. They are registered into the Lua VM by `env-bootstrap/`. When adding a new Lua-exposed function, find the most topical sub-crate and add it there rather than piling onto a single module.

### Windowing / font / IO

- `window/` — the OS-window abstraction (X11, Wayland, macOS, Windows). Referenced via the `window` workspace dependency.
- `wezterm-font/` — font discovery, shaping (HarfBuzz), fallback, glyph rasterization.
- `pty/` (`portable-pty`), `filedescriptor/`, `wezterm-ssh/`, `wezterm-uds/`, `codec/`, `wezterm-client/`, `wezterm-mux-server-impl/` — IO plumbing and the wire protocol between GUI and mux server.

### Docs

User docs live in `docs/` and are built with mkdocs via `ci/build-docs.sh` (runs in a podman/docker container). `ci/generate-docs.py` generates reference pages from source before mkdocs runs. Changelog entries for each PR go in `docs/changelog.md`.

## Conventions

- Format with **nightly** rustfmt (`make fmt` / `cargo +nightly fmt`). CI enforces this via `.github/workflows/fmt.yml`.
- The workspace pins many dependencies via `[workspace.dependencies]` in the root `Cargo.toml`; prefer `workspace = true` in member `Cargo.toml`s.
- Patch in the vendored `cairo-sys-rs` from `deps/cairo` (see `[patch.crates-io]`); don't pull the upstream version.
- New escape sequences should come with a test in `term/src/test/`. New Lua APIs should come with docs under `docs/config/lua/`.
