# Invader Patrol

A browser-capable, retro 2D arcade shooter written in Rust with
[Macroquad](https://macroquad.rs/).

The project began as a terminal game and has been migrated to a fixed-resolution
pixel-art renderer while keeping the simulation independent from the engine.

## Features

- Five rows of animated invaders with accelerating movement
- Three lives, alien projectiles, collisions, scoring, and progressive waves
- Pause, restart, mute, a title screen, and a game-over screen
- Fixed 60 Hz simulation with rendering independent of frame rate
- Native desktop and WebAssembly builds from the same Rust source
- Responsive letterboxed display and touch controls in the browser

## Controls

| Action | Keyboard |
|---|---|
| Move | Left/Right arrows or A/D |
| Fire | Space |
| Pause | P or Escape |
| Mute | M |
| Start/restart | Enter or Space |
| Quit | Q |

## Run natively

Requirements: a current stable Rust toolchain.

```bash
cargo run
```

The default native build is silent so it does not require platform audio system
libraries. On Linux, enable desktop audio with:

```bash
cargo run --features desktop-audio
```

That feature requires the ALSA development package (`libasound2-dev` on Debian
and Ubuntu). Browser builds include audio automatically.

## Build and run in a browser

```bash
rustup target add wasm32-unknown-unknown
./scripts/build-web.sh
python3 -m http.server 8080 --directory web
```

Open <http://localhost:8080>. The build script copies the Rust WebAssembly
binary and all game assets into `web/`.

The browser page also provides touch controls on touch-capable devices. Browsers
only allow audio to start after a user interaction, so the first sound plays
after the first key press or tap.

### WebAssembly audio compatibility patch

`vendor/quad-snd` is a copy of the `quad-snd` 0.2.8 crate with one added line
(`#[link(wasm_import_module = "env")]`). Rust 1.96 removed the implicit
`--allow-undefined` linker behaviour, which the crate relied on to import the
browser audio functions. The copy is wired in through `[patch.crates-io]` in
`Cargo.toml`; it can be removed once the fix is released upstream. See
[`vendor/quad-snd/PATCH-NOTES.md`](vendor/quad-snd/PATCH-NOTES.md).

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Project layout

```text
src/
├── game.rs    # Engine-independent rules, state, collisions, and unit tests
├── main.rs    # Macroquad loop, input, audio, and fixed-timestep driver
└── render.rs  # Pixel-art renderer, HUD, and screens
assets/        # Sprites, font, sounds, and third-party licenses
web/           # Browser loader and generated deployment files
scripts/       # WebAssembly build helper
vendor/        # Patched quad-snd copy for WebAssembly audio
```

## Asset credits

- **Alternate Space Invaders** sprites by WaffleKilometer — CC0 1.0
- **Press Start 2P** by CodeMan38 — SIL Open Font License 1.1
- **8-Bit Sound Effect Pack (Vol. 001)** by Deva / Shades — CC0 1.0
- Macroquad runtime and WebAssembly loader — MIT OR Apache-2.0

Full source links, selected filenames, and license details are in
[`assets/ATTRIBUTION.md`](assets/ATTRIBUTION.md).

This is an independent game project and is not affiliated with or endorsed by
Taito Corporation. “Space Invaders” is a trademark of Taito Corporation; this
project uses its own title and independently licensed artwork.
