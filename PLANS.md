# Implementation Plan: DeepFilterNet3-LL nice-plug VST3 Stability Revision

This plan is a living document. Keep `Resume Here`, `Progress`, `Decision Log`, `Surprises & Discoveries`, and `Outcomes & Retrospective` current during execution. `UPDATE_PLANS.md` is the user-authored requirements brief and remains unchanged; this file turns it into a bounded, resumable execution specification.

## Overview

Revise the existing Rust plugin into a native Apple Silicon VST3/CLAP plugin based entirely on nice-plug and the official DeepFilterNet v0.5.6 low-latency model. The finished plugin must continuously process mono or stereo host audio through one mono model, handle arbitrary host block sizes, report measured latency, align dry and wet paths, reset to a fresh logical state, stream-resample common non-48 kHz rates, and use the same DSP path during real-time, buffered, and offline operation.

The real-time audio callback will not run `DfTract` directly because v0.5.6's Tract inference allocates internally. A persistent worker will own model inference and both streaming resamplers. The callback will use preallocated fixed-size chunks, lock-free SPSC queues, generation and timestamp matching, and a latency-aligned dry fallback. Offline mode may wait with a finite deadline for the same worker output; it does not use a second renderer or DSP implementation.

## Resume Here

- Updated: 2026-08-11 12:44Z
- Overall status: PAUSED — local release package prepared; publication deferred
- Active phase: Release preparation complete
- Active step: None
- Last verified checkpoint: Release Packaging PASS on Step 5.G execution 2/2; Final Acceptance PASS on attempt 2/3.
- Completed since previous checkpoint: Created the current MIT Apple Silicon package and SHA-256 sidecar. The accepted ZIP contains VST3, CLAP, an English installation/security/usage README, project MIT license, third-party notices and separated Apache-2.0 text, release metadata, and binary checksums. Repository screenshot/audio assets are excluded.
- In progress: None. No release upload, plug-in installation, or host mutation was performed.
- Next action: After upstream confirms pretrained-model redistribution terms, the user may attach the ZIP and `.sha256` sidecar to a GitHub Release. The deferred manual Resolve flow remains separately available.
- Blockers / decisions needed: Manual SC-11 evidence remains deferred. Upstream DeepFilterNet issue #697 still leaves pretrained-model redistribution terms unconfirmed, so the package may be prepared locally but must not be described as legally cleared or uploaded by Codex.
- Final verification: PASS — attempt 2/3; exact `cargo xtask bundle deepfilter-vst --release` exited 0 at 2026-08-11 12:41Z and recreated both release bundles after the MIT manifest change. Eight previously recorded non-fatal warnings remain.
- Working tree state: `/Users/shuichi/Documents/GitHub/DeepFilterNet3-VST3`; branch `updatenow`; HEAD `e839c91d8eae4ac46091cd332658621ddbd4609b`; modified: `.gitignore`, `PLANS.md`, `README.md`, `THIRD_PARTY_NOTICES.md`, `plugin/Cargo.toml`; renamed: `LICENSE-MIT` to `LICENSE`, `LICENSE-APACHE` to `third-party-licenses/Apache-2.0.txt`; untracked: `githubreadme/` and `scripts/`. Generated `dist/` is intentionally ignored.
- Evidence: Final attempt 2 completed in 1.77 seconds and recreated both bundles. Step 5.G execution 2/2 exited 0 in 2.42 seconds after adding `ditto --norsrc`; ZIP integrity and SHA-256 verification passed, and inventory inspection found no AppleDouble, PNG, or WAV entries. Accepted archive SHA-256: `5a84c441835bbeefa69c20a301e9c07b3e99a5fc5821b3fa1d35fadb12a36ce8`.

## Execution Contract

- At every session start or resume, read this entire plan and applicable repository instructions before editing.
- Treat Requirements, Boundaries, Steps, and Success Criteria as intended scope. Treat Success Criteria as the verification scope ceiling and the working tree plus bounded verification as evidence of actual completion.
- Reconcile `Resume Here` and `Progress` with repository evidence before choosing work. Preserve unrelated user changes.
- Before implementation, mark exactly one eligible step `IN_PROGRESS` and update `Resume Here`. After every material action or verification, record completed work, remaining work, evidence, and one exact next action.
- Use `[x]` only for a `COMPLETE` step with a UTC timestamp and verification evidence. Keep partial or blocked work unchecked and state the exact remainder or unblock condition.
- Checkpoint before and after risky or long-running operations, before approvals, and before an anticipated pause, handoff, or context compaction. Every stopping point must leave this file truthful if the current agent never returns.
- Do not skip dependencies, silently change scope, or repeat non-idempotent work without first detecting whether it already succeeded.
- Keep at most one step `IN_PROGRESS` unless this plan explicitly declares controlled parallel work and one coordinator owns plan updates.
- Run only verification declared by this plan and mapped to its Success Criteria. Do not add confidence tests, broad regression suites, coverage work, or unrelated fixes.
- Record a final-verification attempt before launching it. Never reset its finite attempt budget on resume or after a repair unless the user changes the contract.
- Treat the first in-budget exit `0` from the Final Acceptance Command as the automated green stop. Run no further automated checks afterward; only a predeclared one-pass manual smoke check that depends on the final artifact may follow.

## Stated Assumptions

1. The target is a personal/open-source Apple Silicon macOS 26.x project, so the `Personal / Lean` verification profile is appropriate.
2. Rust 1.97.1 and Cargo 1.97.1 are the active toolchain. nice-plug 0.2.3 requires Rust 1.87 or later, so the installed toolchain is sufficient.
3. VST3 is the release target, but the existing CLAP export and CLAP ID remain because removing them would be an unnecessary host-compatibility change.
4. The official LL model is the default build. The official standard model remains available only through a mutually exclusive Cargo build feature; it is never a plugin parameter or runtime selection.
5. Supported enhanced rates are integer host rates for which the fixed-output rubato configuration fits the declared maximum host quantum, including 44.1, 48, 88.2, 96, 176.4, and 192 kHz. Any invalid or unsupported configuration initializes as direct, zero-latency bypass instead of failing or producing silence.
6. DaVinci Resolve 20 and pluginval are installed locally. Host validation is still a bounded manual acceptance flow, not a substitute for deterministic Rust tests.
7. No commit, push, release upload, code signing, notarization, or system-wide plugin installation is authorized by this plan.
8. The user selected MIT as the sole project license. Third-party components retain their own licenses and notices.

## Requirements

### R1 — Official build-time model selection

- Pin `deep_filter` to official v0.5.6.
- Set `default-features = false`; enable `tract` and the selected embedded-model feature only.
- Default to `model-ll -> df/default-model-ll`.
- Provide `model-standard -> df/default-model` for comparison/fallback builds.
- Reject both-features and no-feature configurations at compile time.
- Do not add safetensors, MLX, custom ONNX conversion, or another inference backend.

### R2 — Complete nice-plug migration

- Replace all nih-plug dependencies, imports, derives, export macros, xtask dependencies, and bundling instructions with nice-plug equivalents.
- Use released `nice-plug = 0.2.3` and `nice-plug-xtask = 0.1.1`; retain the resolved versions in `Cargo.lock`.
- Preserve plugin name, vendor-facing identity, version semantics, parameter IDs `atten_lim` and `mix`, CLAP ID `com.deepfilter.noise-reduction`, VST3 class ID `DeepFilterNR001\0`, I/O layouts, defaults, ranges, and display semantics.
- Keep `Plugin::HARD_REALTIME_ONLY` false/default so offline rendering remains supported.

### R3 — One-channel inference

- Construct exactly one `DfTract` with `RuntimeParams::default_with_ch(1)`.
- Pass only arrays shaped `[1, model.hop_size]` to the model.
- Mono input passes through unchanged to inference. Stereo input uses `(left + right) * 0.5`.
- Wet output is mono and is copied to both stereo output channels. Each channel's dry contribution remains its own delayed input.

### R4 — One continuous arbitrary-block pipeline

- Decouple host callbacks from model and resampler frame sizes with preallocated chunk accumulation and timestamped lock-free queues.
- For every host input sample, write exactly one host output sample. Never replay a stale wet sample, leave a host sample unwritten, or leak current undelayed dry audio during startup/underflow.
- Use generation tags to discard audio and results from before a reset.
- Real-time and buffered modes never wait for the worker. Late/missing wet output uses the dry sample at the same reported-latency timestamp.
- Offline mode may wait up to 2 seconds per required output chunk, exiting early on worker fault. Timeout uses the same aligned-dry fallback and marks the worker fault; it must not hang the host.

### R5 — Derived, reported, and measured latency

- Derive model latency from the constructed model:
  `df_delay_48k = fft_size - hop_size + lookahead * hop_size`.
- The v0.5.6 LL metadata is expected to be 48 kHz, FFT 960, hop 480, lookahead 0, so intrinsic delay is 480 model-rate samples. Assert metadata rather than substituting a hard-coded 20 ms value.
- Reserve two host quanta for nonblocking worker collection and inference runway.
- For non-48 kHz processing, include both resamplers' `output_delay()` values converted into the correct sample-rate domains.
- Compute and expose a `LatencyBreakdown`; report its total through `InitContext::set_latency_samples()`.
- Delay each dry host channel by exactly that total before applying Mix.
- Empirical impulse tests at 48, 44.1, and 96 kHz must locate the output peak at the reported sample, with at most one sample tolerance for rate conversion.

### R6 — Complete logical reset

- `Plugin::reset()` performs no allocation, locking, model calls, logging, or thread join.
- It increments a generation, clears all callback-owned fixed buffers/cursors/dry delay and parameter smoother history in place, and requests worker reset atomically.
- The worker discards stale generations, resets both resamplers and worker buffers, and replaces the active `DfTract` with a clone of a pristine unprocessed model owned by the worker. Allocation is permitted on that non-audio worker.
- Repeated reset results must match a fresh instance for the same input, including stop/play, seek-style, and repeated-render scenarios.

### R7 — Continuous non-48 kHz handling and safe bypass

- At supported rates, keep persistent rubato 0.14.1 `FftFixedOut<f32>` instances on the worker:
  - host-to-model: fixed output `model.hop_size` at 48 kHz;
  - model-to-host: fixed output equal to the host quantum returned by the first converter.
- Call `process_into_buffer()` with reusable buffers and preserve resampler state between host callbacks.
- Include group delay in R5 and reset both converters for R6.
- If rate validation, model creation, resampler creation, queue sizing, worker startup, or handshake fails, `initialize()` still returns `true`, reports zero latency, and selects unchanged direct bypass.

### R8 — Real-time-safe callback boundary

- Keep `DfTract`, rubato, model reconstruction, heap allocation, and error formatting on the worker.
- The callback may use only bounded linear traversal, preallocated fixed buffers/rings, atomics, lock-free SPSC operations, and parameter reads/smoothing.
- Remove `Mutex`, unsafe manual `Send`/`Sync`, `Vec::push` growth, `Vec::drain`, per-frame `Array2::zeros`, repeated `.nth()`, and callback logging.
- Store attenuation as atomically published bits; the worker calls `set_atten_lim()` only when the effective nonzero value changes.
- At attenuation effectively zero, continue advancing the model with a nonzero internal setting but select a model-rate raw delay matching `df_delay_48k`; do not use `DfTract`'s immediate zero-attenuation return because it breaks alignment/state advancement.
- Model errors use the same aligned raw path and an atomic fault indicator; the callback never panics and never emits silence solely because enhancement failed.

### R9 — Same DSP for real-time and offline

- The same `DspCore`, model, resamplers, queues, timestamps, latency, and reset protocol serve `Realtime`, `Buffered`, and `Offline` process modes.
- Mode affects only whether the bridge waits for a timestamped output; it does not select another model, resampler, renderer, or processing algorithm.

### R10 — Accurate release documentation and notices

- Rewrite README build/install/use/support/latency/model/known-limitations text to match the final implementation and remove placeholder URLs, nih-plug claims, 48 kHz-only claims, and the obsolete Resolve-always-silent warning.
- Document the exact default LL build and exact standard fallback build command.
- Add the repository's declared MIT and Apache-2.0 license texts.
- Add a third-party notice based on the final locked dependency tree and official license files. It must cover at least DeepFilterNet/deep_filter and embedded official model provenance, nice-plug/nice-plug-xtask, rubato, the Rust `vst3` crate, and Steinberg VST3 SDK/trademark guidance where applicable.
- If upstream provides no separate model-weight license file, state that fact and the archive's official repository provenance; do not invent a distinct model license or legal conclusion.

### R11 — Reproducible Apple Silicon release package

- Declare the project itself as MIT while preserving third-party license terms and notices.
- Add an English packaging script that accepts an optional validated version, requires the existing release VST3 and CLAP bundles, verifies thin arm64 architecture plus valid ad-hoc signatures, and never alters the source bundles.
- Create one `DeepFilterNR-v<version>-macos-arm64.zip` containing both plug-in formats, a concise English installation/security README, the project MIT license, third-party notices, a clearly separated Apache-2.0 reference for third-party components, and binary checksums.
- Create a SHA-256 sidecar for the ZIP, refuse to overwrite an existing package, and leave GitHub publishing manual.
- Document the command in README `Development and testing`. State that quarantine removal is solely for users who trust the downloaded archive, and use no broad or `sudo` command.

## Tech Stack and Conventions

- Workspace: Rust/Cargo workspace with packages `deepfilter-vst` and `xtask`.
- Toolchain observed: `rustc 1.97.1`, `cargo 1.97.1`, Apple arm64 macOS 26.
- Framework: crates.io `nice-plug 0.2.3`; exports `nice_export_vst3!` and `nice_export_clap!`; latency API `InitContext::set_latency_samples(u32)`.
- Bundler: crates.io `nice-plug-xtask 0.1.1`; command `cargo xtask bundle deepfilter-vst --release`; artifact `target/bundled/deepfilter-vst.vst3`.
- Inference: official `deep_filter` git tag `v0.5.6` / SHA `978576aa8400552a4ce9730838c635aa30db5e61`; `ndarray 0.15`.
- Resampling: direct exact dependency `rubato = "=0.14.1"`, matching DeepFilterNet v0.5.6; use `FftFixedOut::new(..., sub_chunks = 1, channels = 1)` and `Resampler::process_into_buffer`, `input_frames_next`, `output_frames_next`, `output_delay`, and `reset`.
- Lock-free transport: `rtrb 0.3.3`, preallocated during initialization.
- Tests: Rust unit/integration tests in the plugin crate; real `DfTract` tests plus a deterministic delayed-identity test enhancer.
- Validation tools observed: `/Applications/pluginval.app/Contents/MacOS/pluginval` and `/Applications/DaVinci Resolve/`.
- Code/comments: UTF-8; comments in English; absolute/fully qualified imports where practical; no editing-history comments.

## Boundaries

### Always

- Preserve host-visible IDs and parameter compatibility.
- Keep all model/resampler state on the worker and callback buffers preallocated.
- Preserve unrelated user changes and record any necessary deviation here before implementation.
- Keep standard and LL official archives available through build features without runtime model selection.
- Use only official/primary sources for version and license claims.

### Ask First

- Copy or replace a plugin bundle under `~/Library/Audio/Plug-Ins/VST3` or `/Library/Audio/Plug-Ins/VST3`.
- Restart, rescan, or otherwise mutate user state in DaVinci Resolve when the manual smoke step needs it.
- Delete files, commit, push, publish a release, sign, notarize, or alter CI/deployment.
- Vendor or patch DeepFilterNet/Tract source. The selected worker architecture is intended to avoid that expansion.

### Never

- Convert the model, add safetensors/MLX/custom ONNX, retrain, or replace the tract/libDF inference engine.
- Add a runtime model selector or change VST3/CLAP/parameter IDs without a user-approved plan revision.
- Add a second offline DSP implementation.
- Treat a late worker result as current output or silently increase runtime latency.
- Make plugin initialization failure at a non-48 kHz rate result in silence.

## Success Criteria

- [x] SC-1: The default locked build contains nice-plug and `df/default-model-ll`, contains no nih-plug package/reference, and does not enable `df/default-model`; the explicit standard build compiles only with `--no-default-features --features model-standard`.
- [x] SC-2: The release bundle preserves plugin/CLAP/VST3/parameter identities and is a native arm64 `.vst3` bundle produced at `target/bundled/deepfilter-vst.vst3`.
- [x] SC-3: Tests prove one-channel model shape, stereo downmix/wet redistribution, independent delayed stereo dry, and parameter behavior.
- [x] SC-4: Tests using host partitions `1, 7, 64, 127, 480, 511, 1024` prove exactly one output per input, partition-equivalent output when no worker deadline is missed, explicit startup delay, no stale replay, and aligned-dry underflow fallback.
- [x] SC-5: At 48, 44.1, and 96 kHz, measured impulse latency equals reported latency within the declared tolerance and Mix 0/50/100 peaks remain time-aligned.
- [x] SC-6: Fresh-run output equals output after repeated reset for deterministic and real-model cases; stale pre-reset results never enter a new generation.
- [x] SC-7: Supported non-48 kHz rates use stateful streaming conversion; invalid/unsupported or forced-failure configurations initialize to direct non-silent bypass with zero reported latency.
- [x] SC-8: pluginval strictness 5 passes the debug allocation-asserting VST3, and code inspection confirms no callback locks, heap growth, draining, model/resampler calls, or logging.
- [x] SC-9: Real-time/buffered/offline use the same DSP core; offline waits are bounded and worker fault/timeout produces aligned dry rather than silence or a hang.
- [x] SC-10: README and license/third-party notices match the final locked dependency tree and distinguish verified facts from absent model-specific licensing information.
- [ ] SC-11: The predeclared one-pass DaVinci Resolve 20 flow succeeds on the final bundle: 48 kHz playback and two Deliver renders are non-silent and repeatable across stop/play, seek, bypass toggle, and parameter changes; one 44.1 or 96 kHz playback/Deliver is non-silent; host latency agrees with the recorded impulse result.
- [x] SC-12: The MIT-only packaging workflow produces a verified Apple Silicon ZIP plus SHA-256 sidecar from the rebuilt release bundles, includes both formats and required English guidance/notices, and performs no publication or installation.

## Verification Contract

- Profile: Personal / Lean
- Scope ceiling: Success Criteria SC-1 through SC-11 only.
- Final Acceptance Command: `cargo xtask bundle deepfilter-vst --release`
- Working directory: `/Users/shuichi/Documents/GitHub/DeepFilterNet3-VST3`
- Timeout: 15 minutes
- Maximum final attempts: 3 total; never reset on resume.
- Step checks: Only the exact checks declared by Steps 1.G, 1.3, 3.2, 3.G, and 4.G. Step 3.2 has a user-revised maximum of 4 executions after one sandbox-only failure and a later production startup-bound repair; Step 3.G has a user-revised maximum of 3 executions after both initial attempts stopped at that startup bound before pluginval. All other step checks retain at most 2 total executions. Artifact/code inspection is not a command execution.
- Manual smoke check: After the first successful Final Acceptance Command and only with approval for any user plugin-directory/Resolve mutation, run one bounded DaVinci Resolve 20 flow using the final bundle: at 48 kHz verify mono and stereo playback, Mix/Attenuation changes, bypass toggle, stop/play, and seek; Deliver the same short section twice and compare non-silence/duration/checksum; then repeat playback and one Deliver at either 44.1 or 96 kHz and record the host-displayed latency against the impulse-test value. Run this flow once for the final implementation state.
- Failure policy: Repair only failures attributable to planned changes and within SC-1 through SC-12. Record unrelated findings without fixing them. If the final command cannot pass because it includes an unrelated pre-existing failure, block for plan revision rather than weakening the command.
- Green stop rule: The first in-budget exit `0` ends automated verification. Run no additional tests, lint, typecheck, coverage, build, or review commands afterward; only the predeclared manual smoke check may follow.
- Packaging extension authorized 2026-08-11: changing `plugin/Cargo.toml` to MIT invalidates final attempt 1. Before packaging, record and run final attempt 2/3 using the unchanged exact Final Acceptance Command. Then run `./scripts/package-release.sh`; expected exit 0; timeout 2 minutes; maximum 2 executions. The script's own architecture, signature, ZIP integrity, and checksum checks are the SC-12 gate. Do not rerun Rust tests or pluginval because production DSP code is unchanged.

## Architecture Changes

### Before

- `plugin/src/lib.rs` contains framework adapter, parameters, model, queues, and DSP in one file.
- The audio callback owns `Mutex<Option<DfTract>>`, mutexed growing vectors, allocates `Array2` frames, drains vectors, initializes stereo models for mono data, and does not report or align latency.
- Non-48 kHz initialization returns false.
- xtask and exports are nih-plug-specific.

### After

```text
plugin/src/lib.rs
  DeepFilterPlugin + nice-plug metadata/lifecycle/exports
  ProcessingState::{Active(HostBridge), Bypass}

plugin/src/params.rs
  DeepFilterParams with stable IDs/defaults/formatting

plugin/src/model.rs
  DfEngine (worker-only DfTract)
  ModelInfo { sample_rate, hop_size, fft_size, lookahead, algorithmic_delay }

plugin/src/resampler.rs
  RateConverter::{Identity, Rubato}
  persistent FftFixedOut instances and LatencyBreakdown

plugin/src/dsp.rs
  DspCore: host-rate mono chunk -> resample -> model/raw-delay -> resample -> host-rate wet chunk

plugin/src/worker.rs
  persistent worker, pristine model, generation protocol, SPSC queues, fault/ready atomics

plugin/src/bridge.rs
  arbitrary host blocks, mono downmix, dry delay, timestamp matching, Mix, mode wait policy
```

The worker owns `DspCore`, `DfTract`, pristine `DfTract`, both resamplers, ndarray frames, and reusable resampler buffers. It accepts fixed-capacity `AudioChunk` values tagged with `generation`, `start_sample`, and `len`; output chunks retain the source timeline. `MAX_HOST_QUANTUM` must cover the largest supported converter input/output quantum through 192 kHz (at least 1920 samples) and initialization selects bypass if computed sizes exceed it.

`HostBridge` derives queue capacity from `BufferConfig::max_buffer_size`, the host quantum, two-quanta runway, and two safety chunks. It maintains absolute input/output sample counters. It only consumes an output chunk when generation and expected timestamp match; older output is discarded and newer output stays queued. Startup and missing wet data use the dry-delay sample, which is initially zero and later exactly timestamp-aligned.

At 48 kHz, expected LL components are intrinsic DF delay 480 samples plus two 480-sample host quanta, for a derived nonblocking baseline of 1440 samples before empirical confirmation. At resampled rates:

```text
core_delay_host =
  round((host_to_model.output_delay + df_delay_48k) * host_rate / 48000)
  + model_to_host.output_delay

reported_latency = core_delay_host + 2 * host_quantum
```

The implementation must calculate from live model/converter fields, record the breakdown in tests, and make the impulse position authoritative if an off-by-one rounding discrepancy appears.

## Agent Summary

| Agent | Step Count | Phases Involved |
|---|---:|---|
| `devops-agent` | 3 | 1, 4 |
| `refactoring-agent` | 1 | 1 |
| `coding-agent` | 6 | 2, 3 |
| `review-agent` | 4 | 1, 2, 3, 4 |
| `documentation-agent` | 2 | 4 |

Only the coordinating main agent writes this plan. Write-based implementation remains sequential.

## Implementation Steps

### Phase 1 — Framework, dependency, and identity migration

#### Step 1.1 — Replace framework and model dependency configuration

- **Agent:** `devops-agent`
- **Location:** `Cargo.toml`, `plugin/Cargo.toml`, `xtask/Cargo.toml`, `xtask/src/main.rs`, `Cargo.lock`
- **Action:** Replace nih-plug/nih-plug-xtask with released nice-plug/nice-plug-xtask, add mutually exclusive model features, add exact rubato 0.14.1 and rtrb 0.3.3 direct dependencies, and update the lockfile.
- **Details:** Keep package name `deepfilter-vst`. Define default `model-ll`, optional `model-standard`, and compile errors for both/neither. Configure `df` v0.5.6 with `default-features = false`, base `tract` feature, and feature forwarding. Do not enable DeepFilterNet `transforms` unless required independently after rubato is direct; `tract` already includes required transforms. Use the nice xtask entry point unchanged in shape.
- **Dependencies:** None
- **Verification:** Bounded manifest/lock artifact inspection for SC-1/R1/R2; compilation is deferred to Step 1.G.
- **Complexity:** Medium
- **Risk:** Medium — feature unification can silently embed both large models or retain nih-plug transitively.
- **Idempotence & Recovery:** Safely re-runnable. On interruption inspect manifests plus `cargo tree -e features -p deepfilter-vst`; never hand-edit unrelated lock entries. Restore a coherent manifest first, then regenerate `Cargo.lock` with Cargo.

#### Step 1.2 — Port the plugin adapter and parameters to nice-plug

- **Agent:** `refactoring-agent`
- **Location:** `plugin/src/lib.rs`, new `plugin/src/params.rs`
- **Action:** Port imports, derives, traits, contexts, status types, and export macros to nice-plug while preserving all host-visible identity and parameter behavior.
- **Details:** Keep mono then stereo layouts, IDs/defaults/ranges/formatters, VST3 class bytes, CLAP ID, names, URL unless corrected to this repository's real URL during documentation, and both exports. Initially keep processing as safe direct bypass scaffolding; do not carry over mutex/unsafe wrapper/vector DSP. `initialize()` must return true and report zero latency until Phase 2 installs `HostBridge`.
- **Dependencies:** Step 1.1
- **Verification:** Artifact inspection compares constants and parameter declarations against the original committed `plugin/src/lib.rs`; Step 1.G covers compilation and SC-2.
- **Complexity:** Medium
- **Risk:** Medium — accidental ID or parameter changes can invalidate host sessions.
- **Idempotence & Recovery:** Safely re-runnable. Before replacing declarations, record/compare exact IDs. If interrupted, retain a compile-coherent direct-bypass adapter rather than mixing frameworks.

#### Step 1.3 — Prove the standard build-time fallback

- **Agent:** `devops-agent`
- **Location:** `plugin/Cargo.toml`, `Cargo.lock`
- **Action:** Compile the explicit standard-model configuration and inspect feature resolution so the standard model remains selectable only at build time.
- **Details:** Default features must be disabled for this check. Both/neither configurations are expected compile errors and do not need separate command executions; inspect the guards.
- **Dependencies:** Steps 1.1, 1.2
- **Verification:** From workspace root run `cargo check -p deepfilter-vst --no-default-features --features model-standard`; expected exit 0; timeout 10 minutes; maximum 2 executions; maps to SC-1.
- **Complexity:** Low
- **Risk:** Low

#### Step 1.G — Framework migration phase gate

- **Agent:** `review-agent`
- **Location:** Entire Cargo workspace and nice-plug adapter
- **Action:** Confirm default LL workspace compilation and inspect that no nih-plug reference remains.
- **Dependencies:** Steps 1.1, 1.2, 1.3
- **Verification:** From workspace root run `cargo check --workspace`; expected exit 0; timeout 10 minutes; maximum 2 executions. Then bounded read-only `rg -n "nih[_-]plug|nih_export" . --glob '!target/**' --glob '!UPDATE_PLANS.md' --glob '!PLANS.md'` must return no implementation/documentation matches. Maps to SC-1 and SC-2. The `rg` observation does not consume another verification-command execution.
- **Complexity:** Low
- **Risk:** Low

### Phase 2 — Worker-owned streaming DSP

#### Step 2.1 — Implement the mono model engine and worker lifecycle

- **Agent:** `coding-agent`
- **Location:** new `plugin/src/model.rs`, `plugin/src/worker.rs`
- **Action:** Implement worker-only `DfEngine`, model metadata/latency derivation, fixed-capacity SPSC message types, pristine-model reset, worker startup handshake, generation/fault/readiness atomics, and bounded shutdown.
- **Details:** Construct `DfTract` and its pristine clone on the worker with one channel. Allocate input/output `Array2` frames once on the worker. A named persistent worker drains generation-tagged input chunks, owns all model calls, publishes timestamped output, and never exposes `DfTract` across threads or uses unsafe `Send`/`Sync`. Parameter attenuation is published via `AtomicU32::to_bits/from_bits`. Worker reset clones the pristine model and clears its own buffers before acknowledging the new generation. Shutdown occurs during reinitialize/deactivate/drop, never `process()`/`reset()`.
- **Dependencies:** Step 1.G
- **Verification:** Bounded source inspection for R1/R3/R6/R8; behavioral proof is deferred to Phase 3.
- **Complexity:** High
- **Risk:** High — thread ownership, lifecycle, and stale-generation races can cause hangs or state leakage.
- **Idempotence & Recovery:** Keep creation in a constructor that either returns a fully handshaken worker or an error. On interruption, an absent/failed worker must leave the plugin in `Bypass`. Stop/join any worker created by the current implementation before replacing its queue topology; inspect thread stop/fault atomics before retry.

#### Step 2.2 — Implement the host bridge, timeline, and aligned mix

- **Agent:** `coding-agent`
- **Location:** new `plugin/src/bridge.rs`
- **Action:** Implement arbitrary-block accumulation, mono downmix, stereo dry delay, absolute timestamps, exact output writes, real-time fallback, offline bounded wait, and per-sample Mix smoothing without allocation.
- **Details:** Use fixed-capacity `AudioChunk` arrays and preallocated ring/delay storage sized in initialization. Process channel data linearly. For each host sample, enqueue mono input into the current host quantum, write original mono/stereo samples into the dry ring, and output the delayed dry mixed with the wet mono for the matching timestamp. Startup uses zero-initialized delay. Discard stale output; never consume newer-than-expected output. Realtime/Buffered do not wait. Offline waits at most 2 seconds for the needed chunk and exits on fault/stop/generation change. Queue overflow or lateness uses aligned dry for that exact timestamp.
- **Dependencies:** Step 2.1
- **Verification:** Bounded source inspection for R3/R4/R5/R8/R9; Phase 3 tests cover behavior.
- **Complexity:** High
- **Risk:** High — off-by-one timestamps or partial chunks can duplicate, omit, or shift audio.
- **Idempotence & Recovery:** Encapsulate counters and queue ownership in `HostBridge`; constructor failure yields `Bypass`. Keep reset local and deterministic. If interrupted, document which timestamp invariant is implemented and keep unimplemented output paths returning aligned dry, never partially queued wet.

#### Step 2.3 — Add persistent worker-side resampling and latency breakdown

- **Agent:** `coding-agent`
- **Location:** new `plugin/src/resampler.rs`, `plugin/src/dsp.rs`, updates to `plugin/src/model.rs` and `plugin/src/worker.rs`
- **Action:** Implement identity/streaming conversion, worker `DspCore`, aligned raw-delay fallback, and calculated `LatencyBreakdown` for supported host rates.
- **Details:** For non-48 kHz, create `FftFixedOut<f32>` host-to-model with fixed 480-sample model output and a matching model-to-host converter with fixed host-quantum output. Allocate all planar mono buffers on the worker. Keep state continuous. Model/process errors and attenuation zero select the raw signal delayed by the model's intrinsic delay; still call the model with a nonzero effective attenuation to advance internal state. Calculate converter group delays, two host quanta, and model delay using checked arithmetic; reject non-finite/non-integer/unrepresentable rates or quantum sizes beyond `MAX_HOST_QUANTUM` into bypass.
- **Dependencies:** Steps 2.1, 2.2
- **Verification:** Bounded inspection of converter construction/reset/process calls and checked latency formula for R5/R7/R8; Phase 3 impulse tests are authoritative.
- **Complexity:** High
- **Risk:** High — converter domains and startup group delay are easy to count in the wrong rate, and resampler buffering must remain continuous.
- **Idempotence & Recovery:** Converter creation is transactional inside worker startup. On any error drop the incomplete worker and use direct bypass. Preserve the live formula and test evidence in `LatencyBreakdown`; never patch a failing impulse test with an unexplained constant.

#### Step 2.4 — Integrate nice-plug initialize/process/reset/deactivate

- **Agent:** `coding-agent`
- **Location:** `plugin/src/lib.rs`, `plugin/src/params.rs`
- **Action:** Connect `ProcessingState::{Active, Bypass}` to the worker/bridge, report latency, implement allocation-free reset, and keep the same pipeline across process modes.
- **Details:** `initialize()` safely shuts down an older worker, validates layout/configuration, constructs/handshakes a new bridge, stores `BufferConfig::process_mode`, reports derived latency, and returns true even on fallback. `process()` delegates active blocks or leaves direct bypass unchanged. `reset()` updates generation and clears callback-owned state only. `deactivate()` stops and joins outside the hot path. Reset parameter smoothers to current values without allocation. No process logging.
- **Dependencies:** Steps 2.1, 2.2, 2.3
- **Verification:** Bounded lifecycle/code inspection for R2/R4/R5/R6/R7/R8/R9; Phase 3 supplies executable evidence.
- **Complexity:** High
- **Risk:** High — nice-plug calls reset after initialize and may reinitialize on process-mode/rate changes.
- **Idempotence & Recovery:** Every initialize replaces state transactionally. If a new worker fails, retain no half-active state and install bypass. Before retrying after interruption, inspect/stop any existing worker through its owned handle rather than spawning duplicates.

#### Step 2.G — DSP integration static phase gate

- **Agent:** `review-agent`
- **Location:** `plugin/src/*.rs`
- **Action:** Review callback boundary, queue ownership, generation/timestamp invariants, failure-to-bypass paths, and identity preservation before adding tests.
- **Dependencies:** Steps 2.1, 2.2, 2.3, 2.4
- **Verification:** Bounded artifact inspection mapped to SC-2, SC-4, SC-7, SC-8, and SC-9. Confirm `rg -n "Mutex|unsafe impl|\.drain\(|Array2::zeros|nih_log|nice_log" plugin/src` has no callback-path violation; allowed worker initialization matches must be individually justified in `Surprises & Discoveries`. Behavior remains covered by Step 3.G.
- **Complexity:** Medium
- **Risk:** Medium — static review must catch accidental model/resampler/callback boundary crossings before validation.
- **Idempotence & Recovery:** Review-only and safely repeatable while relevant source is unchanged.

### Phase 3 — Deterministic DSP, reset, latency, and allocation verification

#### Step 3.1 — Add deterministic bridge and failure tests

- **Agent:** `coding-agent`
- **Location:** unit-test modules beside `bridge.rs`, `worker.rs`, `dsp.rs`, and `resampler.rs`
- **Action:** Add a deterministic delayed-identity test engine and tests for partitioning, mono/stereo mapping, dry/wet alignment, queue lateness/overflow, safe bypass, process modes, and generation reset.
- **Details:** Drive identical inputs through partitions `1, 7, 64, 127, 480, 511, 1024`; compare complete output and length. Test Mix at 0%, 50%, and 100%. Force late/fault/stale results and assert aligned dry at the expected timestamp. Force invalid rate/worker construction and assert direct unchanged output and zero latency. Run realtime and offline scheduling against the same core; only the wait policy may differ. A deterministic fake must avoid neural nonlinearity when proving exact samples.
- **Dependencies:** Step 2.G
- **Verification:** Test artifact inspection; execution is Step 3.G. Maps to SC-3, SC-4, SC-6, SC-7, SC-9.
- **Complexity:** Medium
- **Risk:** Medium — asynchronous tests can become flaky unless worker readiness and faults are explicit.
- **Idempotence & Recovery:** Use finite deadlines and deterministic handshakes, never sleeps as correctness conditions. If interrupted, leave each completed test self-contained and list unimplemented SC mappings in Progress.

#### Step 3.2 — Enable and exercise callback allocation detection

- **Agent:** `devops-agent`
- **Location:** `plugin/Cargo.toml`, debug bundle at `target/bundled/deepfilter-vst.vst3`
- **Action:** Build a debug VST3 with nice-plug's `assert_process_allocs` feature so pluginval exercises actual plugin callbacks under allocation abort detection.
- **Details:** This artifact is development-only; release defaults need not enable the feature. Do not add a second allocator framework unless pluginval cannot reach the callback, in which case revise this step before adding dependencies.
- **Dependencies:** Steps 2.G, 3.1
- **Verification:** From workspace root run `cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs`; expected exit 0; timeout 15 minutes; maximum 4 executions as explicitly approved by the user after execution 1 was consumed solely by sandbox denial and a later production startup-bound repair invalidated execution 3's passing artifact. Maps to SC-8 and prepares Step 3.G.
- **Complexity:** Low
- **Risk:** Low

#### Step 3.3 — Add actual-model latency and reset tests

- **Agent:** `coding-agent`
- **Location:** plugin test modules, production `LatencyBreakdown` API
- **Action:** Test official LL metadata, one-channel shape, measured latency at 48/44.1/96 kHz, dry/wet peak alignment, and reset equivalence with the real worker/model.
- **Details:** Use a bounded impulse/non-silent fixture. Set an attenuation limit such as 20 dB so the delayed raw contribution produces an observable impulse. Record the calculated breakdown and locate the output peak; require exact 48 kHz equality and at most one-sample converter tolerance. Compare a fresh engine with reset after prior nonzero audio and two repeated offline runs. Do not assert semantic denoising quality beyond non-silence/timeline behavior.
- **Dependencies:** Steps 2.G, 3.1
- **Verification:** Test artifact inspection; execution is Step 3.G. Maps to SC-3, SC-5, SC-6, SC-7.
- **Complexity:** High
- **Risk:** High — model nonlinearity can make naive peak assertions unstable; the 20 dB raw component and delayed-identity tests separate timing from denoising quality.
- **Idempotence & Recovery:** Tests use fixed inputs and finite deadlines and are safe to rerun within the declared gate budget. If an impulse discrepancy appears, diagnose component delays and rounding; do not add an opaque compensation constant.

#### Step 3.G — DSP and plugin validation phase gate

- **Agent:** `review-agent`
- **Location:** plugin crate tests and debug VST3 bundle
- **Action:** Run the bounded Rust test suite, then validate the already-built allocation-asserting VST3 at pluginval strictness 5.
- **Dependencies:** Steps 3.1, 3.2, 3.3
- **Verification:** One verification-command execution for this gate is the shell command `cargo test -p deepfilter-vst --lib && /Applications/pluginval.app/Contents/MacOS/pluginval --strictness-level 5 --validate-in-process target/bundled/deepfilter-vst.vst3`; expected exit 0; timeout 15 minutes; maximum 3 executions as explicitly approved by the user after executions 1 and 2 both stopped at the former production startup bound before pluginval. Save long output to `/private/tmp/deepfilter-vst-pluginval.log` if needed and record the path. Maps to SC-3 through SC-9.
- **Complexity:** Medium
- **Risk:** Medium — pluginval is an external host process; distinguish code failures from permission/quarantine/tool failures and block rather than weakening strictness.
- **Idempotence & Recovery:** Safe to rerun only when the relevant test/debug artifact changed and budget remains. Detect any still-running pluginval process before retrying. Do not validate a stale bundle; Step 3.2 evidence must match the current source/lock state.

### Phase 4 — Release documentation and licensing

#### Step 4.1 — Rewrite README against verified behavior

- **Agent:** `documentation-agent`
- **Location:** `README.md`, optional update to `Build memo. Mac security issue.md` only if it remains part of supported instructions
- **Action:** Use the repository README workflow to rewrite concise English installation, build, model, rates, latency, parameters, validation, Resolve status, and known-limitations documentation.
- **Details:** Invoke the available `github-readme` skill during execution. Use the real repository URL discovered from git remote. Document default LL and standard build commands, Apple Silicon artifact path, supported enhanced rates and direct-bypass fallback, measured latency table from Step 3.G, mono wet/stereo dry behavior, same real-time/offline pipeline, and any verified worker underrun limitation. Remove nih-plug and obsolete silence warning. Do not claim Windows/Linux/Intel validation that was not performed.
- **Dependencies:** Step 3.G
- **Verification:** Bounded artifact inspection against SC-10 and recorded test evidence; no separate command.
- **Complexity:** Medium
- **Risk:** Low
- **Idempotence & Recovery:** Safely re-editable. Preserve only statements backed by current code/evidence; if implementation changes after documentation, reopen this step.

#### Step 4.2 — Add project licenses and verified third-party notices

- **Agent:** `documentation-agent`
- **Location:** new `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD_PARTY_NOTICES.md`; `Cargo.toml`, `plugin/Cargo.toml`, README license section
- **Action:** Add canonical project license texts and a source-backed notice for the final locked normal dependency/model set.
- **Details:** Keep declared `MIT OR Apache-2.0`. Use `cargo metadata --locked --format-version 1` and normal target dependency resolution as evidence, then inspect official license files. Record: DeepFilterNet/deep_filter v0.5.6 MIT OR Apache-2.0; official LL and optional standard model archive provenance and absence of a separate model-weight license file if still true; nice-plug/nice-plug-xtask ISC; rubato 0.14.1 MIT; `vst3` crate 0.3.0 MIT OR Apache-2.0; Steinberg VST3 SDK MIT and trademark/usage link. Include any other notice-requiring normal dependencies discovered in the final tree. Do not copy licenses for dev-only tools into binary notices unless required.
- **Dependencies:** Steps 1.1, 3.G
- **Verification:** Bounded comparison of notice entries to final `Cargo.lock`, Cargo metadata, and official license sources. Step 4.G runs the sole command. Maps to SC-10.
- **Complexity:** Medium
- **Risk:** Medium — license metadata may be incomplete and the embedded model has no distinct license file in the inspected v0.5.6 tree.
- **Idempotence & Recovery:** Safely re-runnable from the lockfile. If model distribution terms remain materially ambiguous after recording official facts, mark this step BLOCKED and request a user/legal decision rather than guessing.

#### Step 4.G — Documentation and dependency phase gate

- **Agent:** `review-agent`
- **Location:** README, licenses/notices, manifests, lockfile, source tree
- **Action:** Confirm documentation matches code/test evidence and the final dependency tree contains no nih-plug or unintended standard model in the default build.
- **Dependencies:** Steps 4.1, 4.2
- **Verification:** From workspace root run `cargo metadata --locked --format-version 1`; expected exit 0; timeout 5 minutes; maximum 2 executions. Then bounded artifact inspection/`cargo tree -e features -p deepfilter-vst` observation maps dependencies/features to SC-1 and notices to SC-10 without another declared verification-command execution.
- **Complexity:** Low
- **Risk:** Low

### Phase 5 — Apple Silicon release packaging

#### Step 5.1 — Add MIT-only packaging and English release documentation

- **Location:** `plugin/Cargo.toml`, project license files, `.gitignore`, `README.md`, `THIRD_PARTY_NOTICES.md`, `scripts/package-release.sh`, `githubreadme/`
- **Action:** Select MIT as the project license; retain Apache-2.0 only as clearly separated third-party material; add the screenshot and matching effect-off/effect-on WAVs as repository-only README assets; document and implement a non-overwriting release packager with an English user README.
- **Verification:** Static script parse, whitespace inspection, source/asset metadata inspection, and bounded review that images/audio are excluded from the ZIP definition.
- **Status:** COMPLETE — 2026-08-11 12:40Z.

#### Step 5.2 — Rebuild the final release artifacts after the manifest change

- **Action:** Record final attempt 2/3, then run the unchanged Final Acceptance Command once. Do not rerun Rust tests or pluginval because DSP source is unchanged.
- **Verification:** `cargo xtask bundle deepfilter-vst --release`; expected exit 0; timeout 15 minutes; final attempt 2/3.
- **Status:** COMPLETE — 2026-08-11 12:41Z; final attempt 2/3 exited 0 and recreated both bundles.

#### Step 5.G — Create and inspect the local release package

- **Action:** Run the package script once, which verifies the source and staged bundle architectures/signatures, ZIP integrity, and checksums. Confirm the archive inventory excludes repository-only screenshot/audio assets.
- **Verification:** `./scripts/package-release.sh`; expected exit 0; timeout 2 minutes; maximum 2 executions. Follow with read-only archive inventory inspection; no upload or installation.
- **Status:** COMPLETE — 2026-08-11 12:44Z; execution 2/2 passed after the `--norsrc` repair, with ZIP integrity, sidecar SHA-256, and clean inventory evidence.

## Final Acceptance and Manual Host Flow

After every Progress entry through Step 4.G is `COMPLETE`, increment the durable final-attempt counter before running the single Final Acceptance Command. On its first exit 0, set Final verification to PASS and stop all automated verification. Do not rerun Rust tests or pluginval after the green release bundle build.

With approval for any required user plugin-directory and Resolve changes, run the one predeclared DaVinci flow exactly once on that final artifact. Record project sample rates, bundle path/hash, host-reported latency, playback observations, both 48 kHz render hashes/durations/peak levels, and the non-48 kHz render observation. A relevant code/config change after the manual flow invalidates it and consumes another final attempt only if the user authorizes continuation within the remaining budget.

## Risks and Mitigations

1. **Risk:** Tract inference allocates and cannot run in the callback under nice-plug allocation detection.  
   **Mitigation:** Keep model and resamplers entirely worker-owned; plugin callback uses only preallocated transport and aligned fallback.
2. **Risk:** Worker scheduling jitter produces late wet frames.  
   **Mitigation:** Report two host quanta of runway, never wait in real-time, timestamp every chunk, discard stale wet, and substitute the dry sample at the same timestamp.
3. **Risk:** Offline rendering outruns the worker or hangs on a fault.  
   **Mitigation:** Same pipeline with offline-only bounded wait, 2-second per-chunk deadline, fault/stop escape, and aligned-dry fallback.
4. **Risk:** Reset leaves STFT, normalization, rolling, or Tract recurrent state dirty.  
   **Mitigation:** Worker replaces the active model from an unprocessed pristine clone and resets both resamplers; generation tags reject stale output.
5. **Risk:** Latency formula counts resampler delay in the wrong domain or misses frame collection.  
   **Mitigation:** Store a component breakdown with explicit domains, reserve two host quanta, and make 48/44.1/96 kHz impulse positions authoritative within one sample.
6. **Risk:** Attenuation zero invokes DeepFilterNet's immediate undelayed return.  
   **Mitigation:** Continue advancing the model at a nonzero internal limit and select a separate intrinsic-delay-aligned raw wet path.
7. **Risk:** Default Cargo feature unification embeds both official models.  
   **Mitigation:** Disable `df` defaults, use mutually exclusive local features and compile guards, inspect feature tree, and compile the standard fallback separately.
8. **Risk:** Queue capacity is insufficient for a host's maximum block or sample rate.  
   **Mitigation:** Derive capacity from `max_buffer_size`, quantum, runway, and safety chunks with checked arithmetic; unsupported sizes choose direct bypass.
9. **Risk:** nice-plug macOS/VST3 wrapper behavior differs from Rust-only tests.  
   **Mitigation:** pluginval strictness 5 on the debug allocation build and one final DaVinci Resolve flow.
10. **Risk:** Embedded model licensing is not separately stated upstream.  
    **Mitigation:** Record official repository provenance and exact absence of a separate file; do not invent terms, and block public-distribution claims if material ambiguity remains.

## Progress

- [x] Step 1.1: COMPLETE — 2026-08-11 01:40Z; manifests and xtask now use released nice-plug 0.2.3/nice-plug-xtask 0.1.1, local default `model-ll` and optional `model-standard` forward only the selected DeepFilterNet model feature with DeepFilterNet defaults disabled, and the coherent lockfile contains the declared exact rubato/rtrb versions; evidence: `cargo generate-lockfile` exit 0, scoped lock/manifest inspection, no scoped nih-plug matches, and `git diff --check` exit 0. Compile-time both/neither guards will be placed in `plugin/src/lib.rs` during Step 1.2 before Step 1.3.
- [x] Step 1.2: COMPLETE — 2026-08-11 01:44Z; `plugin/src/lib.rs` now implements the nice-plug 0.2.3 adapter with zero-latency direct bypass and exact-one-model guards, while `plugin/src/params.rs` retains both parameter declarations; evidence: bounded comparison preserved name/vendor/URL/version, layout order, CLAP/VST3 IDs and categories, parameter IDs/defaults/ranges/units/formatting/smoothing, and `git diff --check` exited 0. Compilation is deliberately deferred to Step 1.3 and Step 1.G.
- [x] Step 1.3: COMPLETE — 2026-08-11 01:45Z; verification execution 1/2: `cargo check -p deepfilter-vst --no-default-features --features model-standard` exited 0 in 6.13 seconds, proving the explicit standard build-time fallback compiles with defaults disabled.
- [x] Step 1.G: COMPLETE — 2026-08-11 01:46Z; verification execution 1/2 `cargo check --workspace` exited 0 in 0.07 seconds; after narrowly replacing the two planned README framework references, the declared bounded `rg` scan returned no implementation/documentation matches. Phase 1 identity/dependency evidence remains current.
- [x] Step 2.1: COMPLETE — 2026-08-11 01:59Z; `model.rs` derives checked model metadata/delay from one-channel `DfTract` and reuses preallocated frames, while `worker.rs` provides fixed timestamped chunks, two rtrb SPSC queues, handshaken worker-local construction, pristine reset by generation, atomically published attenuation/status/faults, nonblocking host endpoints, and finite non-callback shutdown; evidence: bounded source/callback-boundary inspection and `git diff --check` exit 0. Executable behavior remains deferred to Phase 3 as declared.
- [x] Step 2.2: COMPLETE — 2026-08-11 02:10Z; `bridge.rs` derives bounded queue capacity, accumulates arbitrary host partitions into fixed chunks, preserves mono/stereo mapping and independent dry channels, maps worker stream index plus exactly two host quanta to the host output timeline, matches generation/timestamps without replay, substitutes fully delayed per-channel dry under startup/lateness/fault, waits only in Offline with a two-second chunk deadline, and resets callback storage in place; evidence: bounded source/callback-boundary inspection and `git diff --check` exit 0. Behavioral proof remains deferred to Phase 3.
- [x] Step 2.3: COMPLETE — 2026-08-11 02:24Z; `resampler.rs` preflights exact constant geometry and owns persistent identity/rubato state, `dsp.rs` derives a domain-explicit checked `LatencyBreakdown`, maintains intrinsic-delay raw state, advances healthy model state at zero attenuation, selects delayed raw on zero/model failure, converts back to the host quantum, and resets every stateful component; worker and bridge negotiate the live host quantum/latency and distinguish model degradation from fatal failure; evidence: bounded formula/API/callback-boundary inspection and `git diff --check` exit 0. Impulse authority remains Phase 3.
- [x] Step 2.4: COMPLETE — 2026-08-11 02:31Z; `lib.rs` now transactionally selects `Active(HostBridge)` only after checked layout/rate/queue/worker/DSP/bridge setup, otherwise returns true in direct zero-latency Bypass; active processing delegates mono/stereo to the same bridge, publishes smoothed attenuation, advances per-sample Mix smoothing, reset only resets smoothers/fixed bridge state and requests a generation, and shutdown/join occurs only in initialize/deactivate/drop; evidence: bounded lifecycle/identity/callback inspection and `git diff --check` exit 0. Executable evidence remains Phase 3.
- [x] Step 2.G: COMPLETE — 2026-08-11 02:51Z; independent bounded review found and repairs addressed generation-reset loss, acknowledgement TOCTOU, input timestamp discontinuity, queue-memory bounds, and full-old-queue post-reset ordering. Final re-review reported no scoped material issue; the declared forbidden-pattern scan returned no matches and `git diff --check` exited 0. Behavioral proof remains Phase 3.
- [x] Step 3.1: COMPLETE — 2026-08-11 03:01Z; bounded artifact inspection confirmed the deterministic delayed-identity bridge fake, all seven host partitions with exact output length/equivalence, mono/stereo downmix-wet/dry mapping, per-sample Mix 0/50/100, aligned-dry late/full/fault/stale/future behavior, fresh-equivalent generation reset, mode-only wait policy, checked worker/rate/latency helpers, and plugin-level invalid-construction direct bypass with zero latency. Execution is deliberately deferred to the declared gates.
- [x] Step 3.2: COMPLETE — 2026-08-11 04:18Z; user-approved execution 4/4 of `cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs` exited 0 in 2.68 seconds and recreated current-source debug CLAP/VST3 bundles after the startup-bound repair. The earlier passing artifact was correctly treated as invalid after production changed.
- [x] Step 3.3: COMPLETE — 2026-08-11 04:10Z; bounded artifact inspection confirmed live official LL metadata/one-channel shape, bounded 20 dB finite non-silence, real-core reset and repeated-reset equality after dirty audio, real Offline live-breakdown impulse timing at 48/44.1/96 kHz, Mix 0/50/100 alignment, healthy generation acknowledgement, and two identical post-reset Offline runs. Execution remains the Step 3.G gate.
- [x] Step 3.G: COMPLETE — 2026-08-11 04:27Z; user-approved execution 3/3 of the exact combined command exited 0. All 24 Rust library tests passed in 22.09 seconds, and pluginval strictness 5 printed `SUCCESS` after processing/state/automation/editor/parameter/bus checks across 44.1/48/96 kHz on the current allocation-asserting VST3. Its optional external VST3-validator subtest was skipped because no validator path is configured.
- [x] Step 4.1: COMPLETE — 2026-08-11 04:30Z; the required GitHub README workflow produced a source-backed English README with the real remote, exact model build commands, verified rate/latency/parameter/mapping/fallback behavior, Phase 3 validation, pending Resolve status, and scoped limitations. Artifact inspection removed placeholders, nih-plug, the 48 kHz-only claim, unsupported positive platform claims, and the obsolete guaranteed-silence warning; `git diff --check` exited 0. The workflow's language table was intentionally omitted to preserve the committed deletion of absent `README_ja.md`.
- [x] Step 4.2: COMPLETE — 2026-08-11 04:35Z; added canonical `LICENSE-MIT` and `LICENSE-APACHE`, source-backed `THIRD_PARTY_NOTICES.md`, and README links. Compared pinned manifests/license files for DeepFilterNet v0.5.6, nice-plug/xtask, rubato, rtrb, ndarray, log, `vst3`, and Steinberg VST 3; inspected both embedded DeepFilterNet3 model archives and found no separate license/notice file, so the notice records provenance and absence without a legal inference. `git diff --check` exited 0.
- [x] Step 4.G: COMPLETE — 2026-08-11 04:39Z; verification execution 1/2 of exact `cargo metadata --locked --format-version 1` exited 0 in 0.09 seconds. The bounded feature-tree observation proved default `model-ll -> df/default-model-ll`, no `df/default-model`, and no nih-plug package/reference. A locked-license-family comparison added explicit CC0, Unicode-3.0, BlueOak, 0BSD, BSD-2-Clause, Zlib, Unlicense, Apache-only, and dual-required transitive entries; final docs/notices inspection and `git diff --check` passed.
- [x] Step 5.1: COMPLETE — 2026-08-11 12:40Z; project metadata and README now select MIT, the former Apache project text is retained only under `third-party-licenses/`, the packaging script and English user guidance are implemented, and the supplied screenshot plus matching 48 kHz/24-bit/mono WAV demos are repository-only assets. Static script parsing and `git diff --check` exited 0; bounded inspection confirmed the ZIP definition contains neither image nor audio assets.
- [x] Step 5.2: COMPLETE — 2026-08-11 12:41Z; Final Acceptance attempt 2/3 of exact `cargo xtask bundle deepfilter-vst --release` exited 0 in 1.77 seconds and recreated the current-source ad-hoc-signed VST3 and CLAP bundles. The same eight non-fatal warnings remain; DSP tests and pluginval were not repeated because no DSP source changed.
- [x] Step 5.G: COMPLETE — 2026-08-11 12:44Z; execution 1/2 was rejected only after inventory inspection found AppleDouble entries. After the documented `ditto --norsrc` repair, execution 2/2 exited 0 in 2.42 seconds, the sidecar verified, and inventory contained only both plug-in bundles plus English README, MIT/third-party license material, release metadata, and checksums. No PNG/WAV, upload, or installation occurred. Accepted ZIP SHA-256: `5a84c441835bbeefa69c20a301e9c07b3e99a5fc5821b3fa1d35fadb12a36ce8`.
- [ ] Manual Resolve host flow: DEFERRED_BY_USER — 2026-08-11 04:43Z; the user will conduct the DaVinci Resolve 20 test later. Codex did not copy/install the bundle or operate/restart Resolve. SC-11 remains the only open success criterion.

## Decision Log

- **Decision:** Preserve `UPDATE_PLANS.md` as the requirements brief and author executable state in `PLANS.md`.  
  **Rationale:** The named file contained requirements but no resumable state, step dependencies, success criteria, verification contract, or final command. The implementation-planner workflow requires those before source changes.  
  **Date/Author:** 2026-08-11 / Codex main.
- **Decision:** Use a persistent worker for `DfTract` and resampling rather than synchronous callback inference.  
  **Rationale:** DeepFilterNet v0.5.6 performs allocation-capable Tract operations and even a `spec_ch.to_owned()` inside `process`; direct callback inference cannot satisfy R8.  
  **Date/Author:** 2026-08-11 / Codex main with architecture review.
- **Decision:** Use one worker DSP for all process modes; only waiting policy changes.  
  **Rationale:** This satisfies offline determinism without introducing a second renderer while keeping real-time nonblocking.  
  **Date/Author:** 2026-08-11 / Codex main.
- **Decision:** Report intrinsic/model/resampler delay plus two host quanta, then confirm by impulse.  
  **Rationale:** One quantum collects a fixed frame and a second provides nonblocking inference runway; the official 20 ms LADSPA path can block and is not the same real-time contract.  
  **Date/Author:** 2026-08-11 / Codex main with architecture review.
- **Decision:** Use released nice-plug 0.2.3/xtask 0.1.1 and exact rubato 0.14.1.  
  **Rationale:** Avoid moving framework APIs and share the maintained API line already resolved by DeepFilterNet v0.5.6.  
  **Date/Author:** 2026-08-11 / Codex main and official-docs research.
- **Decision:** Do not vendor or patch DeepFilterNet/Tract in this plan.  
  **Rationale:** Worker ownership solves callback allocation and reset reconstruction while staying on official v0.5.6 and avoiding an inference-engine fork.  
  **Date/Author:** 2026-08-11 / Codex main.
- **Decision:** Increase only Step 3.2's verification maximum from two to three executions.
  **Rationale:** The user explicitly approved one additional identical execution because the first attempt was consumed solely by a frequent sandbox denial; the second reached compilation and produced the now-repaired type diagnostic. All other step and final budgets remain unchanged.
  **Date/Author:** 2026-08-11 / User and Codex main.
- **Decision:** Increase Step 3.2 from three to four executions and Step 3.G from two to three executions.
  **Rationale:** The user explicitly approved one current-source allocation-bundle rebuild and one combined gate rerun after the first two Step 3.G attempts proved the former two-second production startup bound was too short and stopped before pluginval. All other step and final budgets remain unchanged.
  **Date/Author:** 2026-08-11 / User and Codex main.

## Surprises & Discoveries

- `UPDATE_PLANS.md` is 292 lines of detailed intent but had no executable plan state or bounded verification contract.
- The current repository has no tests, CI, `LICENSE`, or third-party notice files; all plugin/DSP behavior is in `plugin/src/lib.rs`.
- `deep_filter` v0.5.6 enables the standard model in its default features. Adding LL without `default-features = false` embeds both, although `DfParams::default()` chooses LL first.
- Official LL config is 48 kHz / FFT 960 / hop 480 / zero model lookahead. The intrinsic delay formula yields 480 samples (10 ms); the official LADSPA README's 20 ms minimum includes its streaming/host pipeline and must not be hard-coded as model latency.
- `DfTract` has no public complete reset. `DfTract::init()` allocates/rebuilds rolling tensors but does not prove full Tract/normalization reset; `DFState::reset()` only clears analysis/synthesis overlap. A worker-owned pristine clone is therefore required.
- The current audio callback creates a stereo model but passes `[1, hop]`, locks three mutexes, allocates frames, grows/drains vectors, traverses buffers with repeated `.nth()`, leaves underfilled outputs untouched, mixes undelayed dry, reports no latency, and returns false outside 48 kHz.
- `DfTract::process()` contains allocation-capable operations, including `spec_ch.to_owned()` before synthesis. nice-plug's callback allocation feature makes direct inference unsuitable.
- pluginval and DaVinci Resolve are installed locally, enabling the bounded host checks in this plan.
- HEAD `af6b7a8` already commits both the new `UPDATE_PLANS.md` and deletion of `README_ja.md`; these are not uncommitted changes to restore.
- The start/resume audit found `PLANS.md` itself was committed after the recorded checkpoint: current clean HEAD is `e839c91d8eae4ac46091cd332658621ddbd4609b`, so the stale `af6b7a8` working-tree description was bookkeeping drift rather than an implementation change.
- The installed Rust toolchain does not currently include `rustfmt`; an attempted formatting check reported that absence. This does not block the plan's declared Cargo verification commands.
- Pinned DeepFilterNet v0.5.6 `DfTract` is cloneable but not `Send` because its live Tract `SimpleState` contains non-`Send` operation state. Both pristine and active models must therefore be constructed and retained entirely inside the worker closure; no unsafe marker implementation is needed.
- Rust provides no timed `JoinHandle::join`. Worker shutdown publishes stop, unparks only from non-callback lifecycle code, waits up to two seconds for `is_finished()`, joins if complete, and otherwise safely detaches the isolated worker so host reinitialization cannot hang.
- Exact rubato geometry for the LL model yields host quanta/total reported latency: 44.1 kHz `441/1764`, 48 kHz `480/1440`, 88.2 kHz `882/3528`, 96 kHz `960/3840`, 176.4 kHz `1764/7056`, and 192 kHz `1920/7680` samples. These are inspection expectations only; Phase 3 impulse positions remain authoritative.
- Reset publication can race an already loaded worker generation, an old-generation full input ring, and the callback's own post-rejection status observations. Processing the just-popped requested-generation chunk fixes the first race; a callback-owned, preallocated FIFO that retains every nonfatal unavailable chunk removes both queue-pressure races without waiting or allocating. Any true FIFO exhaustion latches input discontinuity and therefore produces aligned dry rather than timestamp-corrupt wet audio.
- Host-reported maximum block sizes can make a mathematically valid queue capacity operationally enormous. Worker construction now rejects configurations whose two rings would exceed a checked 16 MiB bound, selecting direct zero-latency bypass instead of attempting an unbounded allocation.
- Deterministic bridge timing tests require control over result readiness without wall-clock scheduler assumptions. A private statically dispatched `BridgeWorker` seam keeps production on `WorkerHandle` while allowing tests to stage the same delayed-identity output until the Offline-only wait hook is invoked; no alternate production DSP path is introduced.
- The first allocation-asserting bundle attempt was blocked before plugin compilation by workspace sandbox policy when nested Cargo metadata tried to unpack `linux-raw-sys 0.12.1` into the normal user registry. This is an execution-environment failure, but it still consumes one of the Step 3.2 command's two declared executions; the sole retry must use the identical command with cache-write approval.
- The approved second allocation-asserting bundle attempt reached production compilation and found that `expected_output()` compared/subtracted a `usize` runway against the `u64` stream timestamp. The source now performs one checked conversion to `u64`; because the infrastructure failure and compile failure consumed both predeclared executions, proof of the repair requires a narrowly approved budget revision rather than an unrecorded third run.
- Rust's default parallel test runner constructed multiple embedded LL Tract models simultaneously. The direct model/core tests completed, but both real-worker bridge startups exceeded the production two-second handshake before reaching their audio assertions. The repair must serialize only real-model test bodies with a bounded test helper; production startup deadlines and code remain unchanged.
- After serializing all real-model test bodies, both real-worker bridge tests still exceeded the two-second startup handshake while direct debug model construction succeeded. The worker's finite startup bound is therefore too aggressive for an unoptimized LL model on this machine; increasing startup only to ten seconds preserves bounded initialization, while the separate two-second Offline output deadline remains unchanged.
- With the startup bound raised to ten seconds, all 24 tests passed. Measured host-sample latency matched the live breakdown at 48 kHz `1440`, 44.1 kHz `1764`, and 96 kHz `3840`; converted-rate peaks stayed within the declared one-sample tolerance and Mix 0/50/100 remained aligned.
- pluginval strictness 5 passed the allocation-asserting VST3 and exercised 44.1/48/96 kHz block sizes and automation. Its separate optional Steinberg `vst3 validator` subtest reported that no validator path was configured and was skipped; this did not fail the declared pluginval command.
- The pinned v0.5.6 repository contains both embedded DeepFilterNet3 ONNX archives under `models/`, while neither archive contains a license, copying, or notice file of its own. Repository-level MIT/Apache files are present; the third-party notice reports only those observable facts.
- The final release build succeeds with eight warnings confined to private visibility bounds/interfaces and test/status helper dead code. They do not prevent bundle creation and cannot be changed after the green stop without invalidating final/manual evidence.

## Official Source Record

- nice-plug repository/API/license: <https://codeberg.org/RustAudio/nice-plug>; inspected main SHA `7df33d3c7471b1c89db65072cf2556d9d25a4737`, crates `nice-plug 0.2.3`, `nice-plug-xtask 0.1.1`, ISC.
- nice-plug lifecycle and latency: <https://codeberg.org/RustAudio/nice-plug/src/branch/main/crates/nice-plug-core/src/plugin.rs>, <https://codeberg.org/RustAudio/nice-plug/src/branch/main/crates/nice-plug-core/src/context/init.rs>.
- nice-plug STFT/latency and allocation examples: <https://codeberg.org/RustAudio/nice-plug/src/branch/main/examples/stft/src/lib.rs>, <https://codeberg.org/RustAudio/nice-plug/src/branch/main/crates/nice-plug/Cargo.toml>.
- DeepFilterNet v0.5.6 crate/features/model/reset/process: <https://github.com/Rikorose/DeepFilterNet/blob/978576aa8400552a4ce9730838c635aa30db5e61/libDF/Cargo.toml>, <https://github.com/Rikorose/DeepFilterNet/blob/978576aa8400552a4ce9730838c635aa30db5e61/libDF/src/tract.rs>.
- Official latency formula: <https://github.com/Rikorose/DeepFilterNet/blob/978576aa8400552a4ce9730838c635aa30db5e61/libDF/src/bin/enhance_wav.rs>.
- Official LADSPA behavior/reference: <https://github.com/Rikorose/DeepFilterNet/blob/978576aa8400552a4ce9730838c635aa30db5e61/ladspa/src/lib.rs>, <https://github.com/Rikorose/DeepFilterNet/blob/978576aa8400552a4ce9730838c635aa30db5e61/ladspa/README.md>.
- rubato 0.14.1 API/license: <https://github.com/HEnquist/rubato/tree/97ca02ba3ac16cd6effe54885866ff30db346b95>.
- Rust `vst3` crate 0.3.0: <https://docs.rs/crate/vst3/0.3.0>.
- Steinberg VST3 SDK license/usage: <https://github.com/steinbergmedia/vst3sdk#license--usage-guidelines>.
- pluginval headless validation: <https://github.com/Tracktion/pluginval>.

## Outcomes & Retrospective

- Automated implementation and release acceptance are complete. The default locked bundle uses nice-plug and official DeepFilterNet v0.5.6 LL only; the standard model remains an explicit alternate build feature.
- Verified reported/measured latency is 1764 samples at 44.1 kHz, 1440 at 48 kHz, and 3840 at 96 kHz. Tests and pluginval cover mapping, reset, aligned fallback, allocation assertions, non-48 kHz conversion, and shared real-time/offline DSP.
- Final release artifacts were recreated at `target/bundled/deepfilter-vst.vst3` and `target/bundled/deepfilter-vst.clap` on final attempt 2/3 after the MIT manifest change. Hash and direct bundle-architecture record remain part of the approved manual-host evidence because the automated green stop is active.
- The MIT-only Apple Silicon distribution candidate is `dist/DeepFilterNR-v0.1.0-macos-arm64.zip` with SHA-256 sidecar and accepted hash `5a84c441835bbeefa69c20a301e9c07b3e99a5fc5821b3fa1d35fadb12a36ce8`. It was prepared locally only; publishing remains deferred pending upstream model-weight licensing clarification.
- Remaining completion condition: SC-11, the single predeclared Resolve 20 playback and Deliver smoke flow, was deferred by the user for manual execution at a later date.
