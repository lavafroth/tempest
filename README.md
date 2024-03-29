# Tempest

Poor man's linux voice automation.

### Goals
- [x] Wake phrase "Tempest rise" and sleep phrase "Tempest rest"
- [x] Shortcut customization using config files
- [x] Recording built into the binary
- [x] Sending fuzzy questions to an LLM through Ollama's API
- [x] Built-in understanding of fuzzy terms (similar sentences are treated as equal)
- [ ] Static builds

### Getting started

Prerequisites:
- Rust toolchain, either via your package manager or [rustup](https://rustup.rs)
- C++ standard library
- `clang`
- `cmake`
- `pkg-config`
- ONNX Runtime
- Audio library for your OS (linux distros have package names like `alsa-dev`, `alsa-lib` or `alsa`)
- `wget` to download the models

Luckily, if you use NixOS with flakes, you can run `nix develop` in the project directory to get a dev shell with all the dependencies installed.

Note: You might have to add your user to the `uinput` group.

Once you have installed the necessary tools, clone this repo. Run the `./download_models.sh` script to download the models needed for speech recongnition and textual inference.

```
git clone https://github.com/lavafroth/tempest
cd tempest
./download_models.sh
```

You may change the keybindings in the config file in case you are not using GNOME+PaperWM.

Finally, run the following:

```sh
cargo run
```

### Acknowledgements

A huge thank you to these folks in helping me build this tool:

- [guillaume-be](https://github.com/guillaume-be/rust-bert) for the Rust-ready library to interact with BERT models.
- [Josh Habdas](https://cpdeberg.org/vhs) for creating the original C FFI using bindgen.
- [abb128](https://github.com/abb128) for the C implementation of [April ASR](https://github.com/abb128/april-asr).
