# Tempest

Poor man's linux voice automation.

### Goals
- [x] Wake phrase "Tempest rise" and sleep phrase "Tempest rest"
- [ ] Shortcut customization using config files
- [x] Recording built into the binary
- [x] Sending fuzzy questions to an LLM through Ollama's API
- [ ] Static builds

### Getting started

This project was created for fulfilling a personal need and therefore has some opinionated settings (for now).

Prerequisites:
- Rust toolchain, either via your package manager or [rustup](https://rustup.rs)
- `clang`
- `cmake`
- `pkg-config`
- ONNX Runtime
- Audio library for your OS (`alsa-dev`, `alsa-lib` or `alsa` for linux)

Luckily, if you use NixOS, you can run `nix develop` in the project directory to get a dev shell with all the dependencies installed.

Once you have installed the necessary tools, clone this repo.

[Download the April model](https://april.sapples.net/aprilv0_en-us.april) and save it as `model.april` in the project directory.

This project has the voice commands hardcoded to the keybindings of my workspace.
You may optionally change the shortcuts in the `voice_command` function.

Run the following:

```sh
cargo run
```
