# Changelog

## Unreleased

### Added

- Added a continuous one-channel DeepFilterNet worker pipeline for mono and
  stereo hosts, with lock-free callback transport, generation-based resets,
  dry/wet alignment, and bounded offline waiting.
- Added enhanced processing at 44.1, 48, 88.2, 96, 176.4, and 192 kHz with
  reported converter and processing latency; unsupported configurations use
  unchanged zero-latency bypass.
- Added explicit `model-ll` and `model-standard` build features. The official
  DeepFilterNet v0.5.6 low-latency model is the default.
- Added `scripts/package-release.sh` to create a verified, non-overwriting
  Apple Silicon ZIP containing both plug-in formats, an English user README,
  licenses, notices, and SHA-256 checksums.
- Added a plug-in screenshot and matching effect-off/effect-on WAV demos to
  the repository README. These media assets are excluded from release ZIPs.
- Added locked-dependency and embedded-model notices, including the unresolved
  upstream pretrained-model redistribution clarification.

### Changed

- Migrated the VST3/CLAP plugin and bundler from nih-plug to released
  nice-plug packages while preserving plugin, parameter, CLAP, and VST3
  identities.
- Changed the project license from `MIT OR Apache-2.0` to MIT. Apache-2.0 text
  is retained only as clearly separated third-party license material.
- Moved model inference and persistent sample-rate conversion off the audio
  callback and aligned missing or degraded wet output with the delayed dry
  timeline instead of emitting stale or silent audio.

### Fixed

- Made logical reset equivalent to a fresh model run and prevented pre-reset,
  late, or discontinuous worker output from entering a new host generation.
- Made initialization failures select direct bypass instead of rejecting
  non-48 kHz hosts or producing silence.
