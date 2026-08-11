# AGENTS.md

Project instructions for coding agents working in this repository.

## Before changing code

- Read `PLANS.md` for the durable implementation and verification state, and
  use `UPDATE_PLANS.md` as the original requirements brief.
- Preserve the plugin name, CLAP ID `com.deepfilter.noise-reduction`, VST3
  class ID `DeepFilterNR001\0`, and parameter IDs `atten_lim` and `mix` unless
  the user explicitly authorizes an identity migration.
- Respect the verification budgets and green-stop state in `PLANS.md`. A
  code, manifest, or configuration change invalidates the final release and
  deferred manual-host evidence; revise the plan before rerunning acceptance.

## DSP and build invariants

- Use nice-plug and nice-plug-xtask. Do not reintroduce nih-plug.
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
- Continue model advancement at an effectively zero attenuation setting while
  selecting the aligned raw path. Do not use DeepFilterNet's immediate
  zero-attenuation return as host output.
- Supported enhanced rates are 44.1, 48, 88.2, 96, 176.4, and 192 kHz.
  Unsupported rates, invalid layouts, queue limits, or startup failures must
  initialize as unchanged direct bypass with zero reported latency.

## Release packaging

- Keep the project license as MIT in `plugin/Cargo.toml` and root `LICENSE`.
  Treat files under `third-party-licenses/` and `THIRD_PARTY_NOTICES.md` as
  third-party terms, not alternative project licenses.
- After a successful release bundle build, create the Apple Silicon archive
  only with `./scripts/package-release.sh`. The script must keep verifying thin
  arm64 binaries, valid ad-hoc signatures, ZIP integrity, and SHA-256 output;
  do not weaken its non-overwrite behavior or remove `ditto --norsrc`.
- Keep `githubreadme/screensho.png`, `githubreadme/effect-off.wav`, and
  `githubreadme/effect-on.wav` as repository README assets. Do not include
  image or audio assets in the release ZIP.
- Keep generated `dist/` artifacts untracked. Attach both the ZIP and its
  `.zip.sha256` sidecar when a release is eventually published.
