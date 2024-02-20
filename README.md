# Tempest

Poor man's linux voice automation.

### Goals
- [x] Wake phrase "Tempest rise" and sleep phrase "Tempest rest"
- [ ] Shortcut customization using config files
- [ ] Recording built into the binary
- [ ] Static builds

### Getting started

This project was created for fulfilling a personal need and therefore has some opinionated settings (for now).

Prerequisites:
- NixOS (or a Nix infected distro)
- Pulseaudio installed
- GNOME with PaperWM (for now)

Once you have installed the necessary tools, clone this repo.

[Download the April model](https://april.sapples.net/aprilv0_en-us.april) and save it as `model.april` in the project directory.

This project has the voice commands hardcoded to the keybindings of my workspace.
You may optionally change the shortcuts in the `voice_command` function.

Run the following:

```
nix develop --command $SHELL
parec --format=s16 --rate=16000 --channels=1 --latency-ms=100 | cargo r
```
