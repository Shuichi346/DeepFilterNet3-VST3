# AGENTS.md

Project instructions for coding agents working in this repository.

## Before changing code

- Read `PLANS.md` for the durable implementation and verification state, and
  use `UPDATE_PLANS.md` as the original requirements brief.
- Preserve the plugin name, CLAP ID `com.deepfilter.noise-reduction`, VST3
  class ID `DeepFilterNR001\0`, and parameter IDs `atten_lim` and `mix` unless
  the user explicitly authorizes an identity migration.
- Follow the change-specific verification and retry budgets in `PLANS.md`;
  stop once affected acceptance passes. Completed historical gates are not
  prerequisites for unrelated maintenance. Production source, manifest, or
  build-configuration changes invalidate affected release/manual-host evidence;
  test-only edits invalidate affected test evidence, while documentation-only
  edits do not invalidate binaries. Revise the plan before new acceptance work.

## DSP and build invariants

- Use nice-plug and nice-plug-xtask. Do not reintroduce nih-plug.
- Keep nice-plug, nice-plug-egui, and egui on compatible versions when updating
  `plugin/Cargo.toml` and `Cargo.lock`. The current editor requires Rust 1.95
  or later. Use `Plugin::activate`/`ActivateContext` and the concrete
  `Plugin::Editor` associated type for the current framework.
- Treat DeepFilterNet, ndarray, Tract, and rubato as a shared compatibility
  boundary; assess model APIs and resampling/latency behavior before upgrading
  them independently.
- Enable exactly one embedded model feature. The default is `model-ll`, and
  the alternate build is `--no-default-features --features model-standard`.
  Keep DeepFilterNet default features disabled so both models are never
  embedded together.
- Keep `DfTract`, model reconstruction, and persistent rubato converters on
  the worker. The audio callback must not allocate, lock, wait, log, call the
  model, or call a resampler.
- Preserve timestamp and generation matching, per-channel latency-aligned dry
  fallback, and the single shared DSP path for real-time, buffered, and
  offline modes. Only Offline may wait, and its wait must remain bounded.
- Derive collection runway from the negotiated maximum host block plus one
  model quantum. Keep it in reported latency and both output timelines;
  deterministic immediate-worker tests alone do not prove real-time coverage.
- Advance parameter smoothing in audio-sample time. On input queue overflow,
  restart the worker generation without resetting host counters or dry delay,
  and keep aligned dry until the new model/resampler history becomes valid.
- Continue model advancement at an effectively zero attenuation setting while
  selecting the aligned raw path. Do not use DeepFilterNet's immediate
  zero-attenuation return as host output.
- Supported enhanced rates are 44.1, 48, 88.2, 96, 176.4, and 192 kHz.
  Unsupported rates, invalid layouts, queue limits, or startup failures must
  initialize as unchanged direct bypass with zero reported latency.

## Editor invariant

- Keep the custom editor fixed-size and English-only with exactly two
  interactive controls: the existing `atten_lim` and `mix` parameter sliders.
  Do not add model selection, waveforms, meters, bypass, presets, or other
  controls unless the user explicitly expands the UI scope.
- Keep GUI work outside the audio callback and route slider gestures through
  nice-plug's parameter setter so host automation remains synchronized.
- In `plugin/src/editor.rs`, preserve the `NiceEguiApp` lifecycle and the
  framework's `RepaintNotifier` integration so host automation repaints the UI.
  Reset temporary text-entry and drag state when the editor is reopened.

## Release packaging

- Keep the project license as MIT in `plugin/Cargo.toml` and root `LICENSE`.
  Treat files under `third-party-licenses/` and `THIRD_PARTY_NOTICES.md` as
  third-party terms, not alternative project licenses.
- Keep license documentation concise: do not restore dependency-purpose prose
  or license-expression tables. Preserve the embedded-model redistribution
  warning, required MIT/ISC/font notices, canonical Apache-2.0, Unicode, and
  embedded-font texts, and the exact VST trademark attribution in repository
  and package output.
- After a successful release bundle build, create the Apple Silicon archive
  only with `./scripts/package-release.sh`. The script must keep verifying thin
  arm64 binaries, valid ad-hoc signatures, ZIP integrity, and SHA-256 output;
  do not weaken its non-overwrite behavior or remove `ditto --norsrc`.
- Keep `githubreadme/screensho.png`, `githubreadme/effect-off.wav`, and
  `githubreadme/effect-on.wav` as repository README assets. Do not include
  image or audio assets in the release ZIP.
- Keep generated `dist/` artifacts untracked. Attach both the ZIP and its
  `.zip.sha256` sidecar when a release is eventually published.
