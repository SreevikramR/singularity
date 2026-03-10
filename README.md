# Singularity

A unified, comprehensive quick-settings applet for the COSMIC™ desktop environment. Singularity provides quick access to essential system controls like Wi-Fi, Bluetooth, Audio, Power Profiles, and VPN settings, all wrapped in a clean, unified interface.

## Prerequisites

- `rust` and `cargo`
- `just` (command runner)
- A system with standard Linux desktop services:
  - NetworkManager
  - UPower
  - BlueZ
  - PipeWire
  - MPRIS-compatible media players

## Building and Installing

A [justfile](./justfile) is included by default with common recipes used by other COSMIC projects. Install from [casey/just][just]

- `just` builds the applet with the default `just build-release` recipe
- `just run` builds and runs the applet
- `just install` installs the project into the system
- `just vendor` creates a vendored tarball
- `just build-vendored` compiles with vendored dependencies from that tarball
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## Documentation

Refer to the [libcosmic API documentation][api-docs] and [book][book] for help with building applets with [libcosmic][libcosmic].

[api-docs]: https://pop-os.github.io/libcosmic/cosmic/
[book]: https://pop-os.github.io/libcosmic-book/
[cargo-generate]: https://cargo-generate.github.io/cargo-generate/installation.html
[libcosmic]: https://github.com/pop-os/libcosmic/
[just]: https://github.com/casey/just
