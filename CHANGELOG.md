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
- Added a compact English custom editor containing only Attenuation Limit and
  Mix sliders, with host-aware parameter gestures for automation.
- Added the required Apache-2.0, MIT, ISC, and Unicode notices, including the
  unresolved upstream pretrained-model redistribution clarification.
- Added the required embedded GUI font notices and license texts to the
  repository and release-package inventory.

### Changed

- Upgraded nice-plug to 0.4.2, nice-plug-egui to 0.5.1, egui to 0.36.2,
  rtrb to 0.3.5, and log to 0.4.34. Migrated activation and editor integration
  while preserving plugin identities, DSP behavior, and the fixed two-control UI.
- Raised the documented build requirement to Rust 1.95 for the current
  editor dependencies.
- Updated third-party notices for the resolved editor dependencies and
  confirmed that the embedded font license texts remained unchanged.

- Updated the plug-in version from 0.6.0 to 0.7.0.

- Consolidated implementation tracking in `PLANS.md` and replaced repeated
  verification gates with checks scoped to the affected behavior and artifacts.
- Changed the project and plug-in release version to 0.5.0.
- Migrated the VST3/CLAP plugin and bundler from nih-plug to released
  nice-plug packages while preserving plugin, parameter, CLAP, and VST3
  identities.
- Changed the project license from `MIT OR Apache-2.0` to MIT. Apache-2.0 text
  is retained only as clearly separated third-party license material.
- Simplified repository and release-package license sections by removing
  dependency-purpose prose and license-expression tables while retaining
  required notices and attribution.
- Moved model inference and persistent sample-rate conversion off the audio
  callback and aligned missing or degraded wet output with the delayed dry
  timeline instead of emitting stale or silent audio.

### Fixed

- Corrected Attenuation Limit smoothing so a 50 ms change no longer takes
  seconds when the host uses larger blocks.
- Sized the reported processing delay for the host's maximum block size,
  preventing routine large callbacks from losing noise reduction to dry fallback.
- Restored enhancement automatically after input queue overload while preserving
  the stereo dry timeline and rejecting stale worker output.

- Made logical reset equivalent to a fresh model run and prevented pre-reset,
  late, or discontinuous worker output from entering a new host generation.
- Made initialization failures select direct bypass instead of rejecting
  non-48 kHz hosts or producing silence.
