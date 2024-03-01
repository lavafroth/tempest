# aprilasr-sys

Low-level FFI bindings for the [april-asr](https://github.com/abb128/april-asr) C api (libaprilasr).

## Overview

Compiles `libaprilasr` from source to `OUT_DIR` using CMake and generates bindings for April API, made available via vendored upstream source pointing at a specific commit sha as defined by the git submodule.

## Requirements

Building requires the following system libraries:

- libonnxruntime
- libclang

Use `locate` to search for installed libraries. For example, on Linux run command `locate libonnxruntime.so` to determine if the [ONNX Runtime](https://onnxruntime.ai/) is available.

## Installation

To get the latest unyanked release published to [crates.io]:

```sh
cargo add aprilasr-sys
```

Or get the tip of the development branch via cargo using git:

```sh
cargo add --git https://codeberg.org/vhs/aprilasr-sys.git
```

It's also possible to vendor this crate:

```sh
mkdir crates && \
    (cd crates; git submodule add https://codeberg.org/vhs/aprilasr-sys)
```

And once cloned updating dependent's `Cargo.toml` manifest like:

```toml
[dependencies]
aprilasr-sys = { path = "crates/aprilasr-sys" }
```

## Examples

For a basic usage example see `examples/init.rs` and run command:

```
cargo run --example init
```

You should see output like:

```term
April ASR api v1 initialized and ready for model.
```

## Development

First clone vendored source:

```sh
git submodule update --init --recursive
```

To generate bindings run command:

```sh
cargo build [--release]
```

To specify include directory set env `APRIL_INCLUDE_DIR` before running build.

To inspect bindings generated:

```sh
bat $(echo $(exa target/*/build/*/out/bindings.rs) | head -1)
```

Command requires `bat` and `exa` rust binaries and displays output with syntax highlighting.

## Versioning

Consider using `chrono` to parse the date format unless april-asr adopts [semantic versioning](https://semver.org/):

```rust
let date_str = "2023.05.12";
let native_date = chrono::NaiveDate::parse_from_str(date_str, "%Y.%m.%d");
p!("{:?}", native_date);
```

Here `p!` is a debug helper in `build.rs` and `date_str` represents the `VERSION` in `vendor/april-asr/CMakeLists.txt` file. With some additional work [cmake-parser](https://crates.io/crates/cmake-parser) looks well-suited for parsing the file to get the version.

Date-based versioning is not currently implemented in the `build.rs` build script. Once versioning is implemented it would also ideal to use it as an input to [pkg-config](https://crates.io/crates/pkg-config) to scan the system for the library.

See [Making a \*-sys crate](https://kornel.ski/rust-sys-crate) for other possible enhancements.

## Vendoring

Because we are vendoring April ASR source using a git submodule we have the ability to update the submodule to generate new bindings when the upstream source code changes.

To view the current commit of the `april-asr` submodule:

```sh
git submodule status | awk '{print substr($1, 1, 7)}'
```

To update the submodule to the latest commit in the `main` branch of the submodule:

```sh
git submodule update --remote --recursive --merge
```

This command fetches the latest commits from the submodule's remote repository, checks out the commit referenced by the `main` branch, and updates the submodule in the main repository.

## Releasing

Steps to package a release of this crate for [crates.io]:

1. Update git submodule as described in [Vendoring](#vendoring).
1. Run `cargo clean` to remove existing build artifacts.
1. Run `cargo build --release` to update generated bindings.
1. Inspect bindings as described in [Development](#development).
1. Run `cargo test` to execute unit tests including bindgen.
1. Run `cargo run --example init` to check example.
1. Run `cargo doc` to generate crate docs and review them.
1. Bump the package version in `Cargo.toml` manifest.
1. Run `cargo publish --dry-run` to review your work.
1. Then `cargo login` and `cargo publish` to publish crate.

Once published visit [docs.rs] to review [crate documentation](https://docs.rs/aprilasr-sys/).

[crates.io]: https://crates.io/
[docs.rs]: https://docs.rs/
