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

Once you have installed the necessary tools, clone this repo. Run the `download_models.sh` script to download the models needed for speech recongnition and textual inference.

```
git clone https://github.com/lavafroth/tempest
cd tempest
./download_models.sh
```

Change the bindings in the config file to suit your needs.

Run the following to build the daemon and the client:

```sh
cargo build --workspace --release
```

#### Daemon

The daemon is optional and is only needed if you want phrases in your bindings to perform keyboard shortcuts.
Since performing keystrokes is a privileged action, you must run the daemon as root.

```sh
sudo ./target/release/tempest-daemon
```

This will give a token to authenticate with the daemon.

#### Client

If you have the daemon running in the background, in a different terminal tab, run

```sh
./target/release/tempest-client the_token_from_the_daemon
```

where `the_token_from_the_daemon` is the token provided by the daemon.

If you wish to opt out of using the daemon, you can run the client standalone. However, config bindings with keyboard shortcuts will not work.

```sh
./target/release/tempest-client
```

#### Note
The april model in the download script might not work properly if you
have a terrible microphone like mine. In that case, you may download an older
model from [here](https://april.sapples.net/aprilv0_en-us.april) and save it as `model.april` in the project directory.
This older model is less accurate in speech recognition but can work with more noisy data.

### Acknowledgements

A huge thank you to these folks in helping me build this tool:

- [Josh Habdas](https://cpdeberg.org/vhs) for creating the original C FFI using bindgen.
- [abb128](https://github.com/abb128) for the C implementation of [April ASR](https://github.com/abb128/april-asr).
