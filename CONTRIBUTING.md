# Contributing

Thanks for your interest in Rhythm Rail. This document covers how to contribute effectively.

## Getting Oriented

Read these first:

1. **[README](../README.md)** — what this project is and how to run it
2. **[Architecture](docs/ARCHITECTURE.md)** — how the code is structured
3. **[Roadmap](docs/ROADMAP.md)** — what's being worked on and what's next

## Ways to Contribute

**Code** — pick an unchecked item from the Roadmap, open an issue to claim it, submit a PR. Small, focused PRs are easier to review than large ones.

**Charts** — once the beat map format stabilizes, creating charts for songs is one of the most valuable contributions. We need test charts of varying difficulty for development, and a library of charts for release.

**Testing** — try the game on different hardware and report issues. Especially valuable: testing different audio backends (WASAPI, ASIO, ALSA, PipeWire, CoreAudio), gamepads, and monitors (for latency characteristics).

**Documentation** — improve existing docs, add examples, write tutorials.

**Bug reports** — file issues with steps to reproduce. For timing/sync bugs, include your OS, audio backend, and frame rate.

## Development Setup

```bash
git clone https://github.com/yourname/rhythm-rail.git
cd rhythm-rail
cargo run  # debug build for iteration
cargo run --release  # release build for testing timing accuracy
```

For audio latency testing on Windows, you may want to enable ASIO support:

```toml
# In Cargo.toml, enable the ASIO feature on cpal if available
```

## Code Style

- Follow standard `rustfmt` formatting (`cargo fmt`)
- Run `cargo clippy` with no warnings
- Use Bevy's ECS patterns: prefer components over inheritance, systems over methods
- Keep systems small and single-purpose
- Document public APIs with doc comments
- Timing-critical code should have comments explaining *why* the approach was chosen

## Pull Request Process

1. Open an issue first for non-trivial changes (discussion saves rework)
2. Fork and create a feature branch
3. Write code, run `cargo fmt` and `cargo clippy`
4. Test with both debug and release builds (timing behavior differs)
5. Open a PR with a description of what changed and why
6. Address review feedback

## Architecture Decisions

If you're proposing a significant architectural change, open an issue first to discuss. Key constraints to keep in mind:

- **Audio clock is the source of truth.** Never accumulate frame deltas for song position.
- **Input events, not polling.** Use `EventReader<KeyboardInput>` / `EventReader<GamepadEvent>`, not `ButtonInput` resources.
- **Offline chart generation.** Analysis happens before gameplay, not during. Real-time FFT is only for visual effects.
- **Beat-based timing.** Charts use beats, not milliseconds. The engine resolves beats to time at runtime.

## License

By contributing, you agree that your contributions will be licensed under the project's MIT / Apache-2.0 dual license.
