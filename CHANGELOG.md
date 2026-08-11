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
- Added project MIT/Apache-2.0 license texts and locked-dependency/model
  notices.

### Changed

- Migrated the VST3/CLAP plugin and bundler from nih-plug to released
  nice-plug packages while preserving plugin, parameter, CLAP, and VST3
  identities.
- Moved model inference and persistent sample-rate conversion off the audio
  callback and aligned missing or degraded wet output with the delayed dry
  timeline instead of emitting stale or silent audio.

### Fixed

- Made logical reset equivalent to a fresh model run and prevented pre-reset,
  late, or discontinuous worker output from entering a new host generation.
- Made initialization failures select direct bypass instead of rejecting
  non-48 kHz hosts or producing silence.
