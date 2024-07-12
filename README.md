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

#### Prerequisites
- Rust toolchain, either via your package manager or [rustup](https://rustup.rs)
- C++ standard library
- `clang`
- `cmake`
- `pkg-config`
- `libvosk` in `LD_LIBRARY_PATH`
- Audio library for your OS (linux distros have package names like `alsa-dev`, `alsa-lib` or `alsa`)

If you use Nix flakes, run `nix develop` in the project directory to get a dev shell with the dependencies installed.

#### Building

```
git clone https://github.com/lavafroth/tempest
cd tempest
```

Change the bindings in the config file to suit your needs.

Run the following to build the daemon and the client:

```sh
cargo build --workspace --release
```

##### Daemon

The daemon is optional and is only needed if you want phrases in your bindings to perform keyboard shortcuts.
Since performing keystrokes is a privileged action, you must run the daemon as root.

```sh
sudo ./target/release/tempest-daemon
```

This will give a token to authenticate with the daemon.

##### Client

If you have the daemon running in the background, in a different terminal tab, run

```sh
./target/release/tempest-client \
the_token_from_the_daemon
```

where `the_token_from_the_daemon` is the token provided by the daemon.

You can alternatively run the client as standalone. However, config bindings with keyboard shortcuts will not work.

```sh
./target/release/tempest-client
```

On first run, the client will prompt you to download models.
