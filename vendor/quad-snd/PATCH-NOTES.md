# Local patch of `quad-snd` 0.2.8

This directory is a copy of the `quad-snd` crate as published on crates.io, with
one small compatibility patch in `src/web_snd.rs`:

```rust
#[link(wasm_import_module = "env")]
extern "C" { ... }
```

Rust 1.96 removed the implicit `--allow-undefined` behaviour that older Rust
toolchains used for WebAssembly links. Without the attribute, building
`macroquad` with its `audio` feature for `wasm32-unknown-unknown` fails with
linker errors such as `undefined symbol: audio_init`.

The patch is applied through `[patch.crates-io]` in the workspace `Cargo.toml`.
It can be removed once an upstream `quad-snd` release includes the same
attribute (or an equivalent fix).

`quad-snd` is distributed under the MIT OR Apache-2.0 license. See the upstream
project for the full license texts.
