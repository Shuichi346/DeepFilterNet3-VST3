# DeepFilterNet3-VST3 Implementation Plan

## Progress Status

This is the sole authoritative checklist. `UPDATE_PLANS.md` remains the original requirements brief; `AGENTS.md` contains repository invariants. Earlier construction history is available in Git and `NOTES.md`.

- [x] I1–I9: Framework migration, worker DSP, resampling, editor, documentation, and packaging implemented. Evidence: current source and recorded acceptance below.
- [x] I10: Sample-time attenuation, maximum-block runway, and automatic overflow recovery implemented and accepted on 2026-09-26.
- [x] V10.1: All 31 Rust library tests passed (22.19 seconds).
- [x] V10.2: Allocation-asserting VST3 passed pluginval strictness 5.
- [x] V10.3: Optimized release passed the operational probe and package verification; approved user VST3 installation completed.
- [x] P11: Current skills, requirements, all 31 tests, and saved results reviewed for this cleanup.
- [x] I11: Consolidated the plan and repository verification rules; removed duplicate gates, trackers, obsolete instructions, and narrative history.
- [x] V11: Documentation inspection and `git diff --check` passed; only `PLANS.md` and `AGENTS.md` changed. No source/tests or release artifacts changed.
- [x] SC-11: Deferred Resolve 21 acceptance on the final artifact; prerequisites and remaining flow below. This is not a prerequisite for plan cleanup or unrelated maintenance.

- [x] I12: Plugin manifest/lock entry and current README packaging examples now use 0.7.0; dependencies and DSP are unchanged.
- [x] V12: Release build passed on execution 1 in 6.25 s; exported CLAP descriptor reports 0.7.0 from the shared plugin VERSION constant. Packaging execution 1 passed architecture/signature/ZIP/SHA-256 checks. Existing DSP tests were not repeated for version-only changes.
- [x] I12.D: Installed the verified v0.7.0 VST3 with matching executable hash; previous installed bundle is preserved at `dist/installed-backup-before-v070/deepfilter-vst.vst3`. No Resolve mutation or publication.

- [x] I13: Upgrade nice-plug 0.4.2, nice-plug-egui 0.5.1, egui 0.36.2, rtrb 0.3.5, and log 0.4.34; migrate lifecycle/editor APIs and retain fixed two-control behavior.
- [x] V13.1: Run all library tests once (20 minutes, at most three executions including focused repairs).
- [x] V13.2: Build allocation-asserting bundles and run pluginval strictness 5 (20 minutes per unit, at most three executions each).
- [x] V13.3: Build optimized bundles, package under an unused dependency-update suffix using the release script, and inspect documentation/notices and diff (20 minutes build; two packaging executions maximum).
- [x] D13: Update current dependency/compiler documentation and record verification and remaining manual-host limits.

Current: Complete — recommended dependency upgrades and automated acceptance.
Next: None for this request. Manual Resolve acceptance remains deferred; installation and publication were not performed.

V13.3/D13: Optimized VST3/CLAP build passed on execution 1 in 1m 50s; packaging passed on execution 1 with thin arm64, ad-hoc signatures, ZIP integrity, and SHA-256 verification. Package excludes README media and contains the updated notices. README versions/compiler requirement, changelog, engineering notes, and license notices were reviewed; `git diff --check` passed. Logs: `/private/tmp/deepfilter-upgrade-release.log` and `/private/tmp/deepfilter-upgrade-package.log`.
Executable SHA-256: `4457eeffad7433d4e6944df1f98a906f1d6f20e06239154c9d98391c9c437d1c`.
Package: `dist/DeepFilterNR-v0.7.0-deps-20260926-macos-arm64.zip` and `.zip.sha256`; archive SHA-256: `3ca36e5d4a5730f7c8a7c89b4641d7d595a9c7de9e58e4e4528b9fd7c8d9db19`.
Plugin version remains 0.7.0; the archive suffix distinguishes this dependency-update build. The prior installed bundle and archives are unchanged. V10 operational-probe measurements are historical; this migration has fresh library and pluginval evidence, not a fresh paced-host probe or manual Resolve result.

V13.2: Allocation-asserting VST3/CLAP bundles built successfully; pluginval strictness 5 finished with SUCCESS on execution 1, including editor, editor automation, state, bus, and multi-rate processing checks. Logs: `/private/tmp/deepfilter-upgrade-debug.log` and `/private/tmp/deepfilter-upgrade-pluginval.log`. Initial bundler metadata cache denial was resolved with permission. Eight existing private-interface/dead-code build warnings remain. External Steinberg validator was unconfigured and skipped.

V13.1: 31/31 tests passed in 21.79 s on execution 2 after a GuiContext import correction; initial cache permission denial did not launch compilation. Log: `/private/tmp/deepfilter-upgrade-tests.log`. Two existing unused-mut warnings in bridge tests remain.

Scope: User authorized the recommended framework/editor and maintenance dependency upgrades on 2026-09-26. Keep DeepFilterNet, ndarray, Tract, rubato, model features, DSP algorithms, identities, and version 0.7.0. Framework changes require fresh automated host evidence; prior installed/released binaries remain historical and manual Resolve acceptance remains deferred. No installation or publication is included. A fresh release build is packaged with a unique suffix to preserve existing archives.

V12 executable SHA-256: `83e3f1e5abe78e9b893c3b344b5c4836fd8c5e85d69f00a071d92935ba32be9d`.
Package: `dist/DeepFilterNR-v0.7.0-macos-arm64.zip` and `.zip.sha256`; archive SHA-256: `b79a8aeea40e02e4a7e79c3adb6e78d984bd49e7f30d8c00f33d2f6fa8060095`.
Version inspection initially expected Cargo's version in Info.plist, but the existing bundler writes 1.0.0 there. The actual host-facing plugin descriptor was then queried directly and confirmed as 0.7.0; no bundler behavior was changed. Earlier artifact hashes below describe historical V10 evidence.

Save this tracker after each implementation step or independent verification unit. Use `[x]` only for evidenced completion. Show the checklist at start, phase boundaries, and completion, with concise changed-item updates otherwise. Reconcile against Git and actual artifacts after interruption; do not restart completed phases.

## Outcome and scope

Maintain a native Apple Silicon macOS VST3/CLAP noise-reduction plugin with official DeepFilterNet3-LL inference, continuous mono/stereo processing, aligned dry/wet output, complete reset, and a fixed English two-control editor. Real-time, buffered, and offline rendering share one DSP implementation.

The current request is the I13 dependency upgrade scoped above. Preserve the existing DSP architecture and product contracts; revalidate the changed framework and editor integration before producing local release artifacts.

Preserve these product requirements:

- Use nice-plug and nice-plug-xtask. Keep the name `DeepFilter Noise Reduction`, CLAP ID `com.deepfilter.noise-reduction`, VST3 class ID `DeepFilterNR001\0`, and parameter IDs `atten_lim` and `mix`.
- Embed exactly one official DeepFilterNet v0.5.6 model. Default `model-ll` forwards `df/default-model-ll`; alternate `model-standard` requires `--no-default-features`. Keep `df` defaults disabled and both/neither guards. No model conversion, new inference backend, or runtime selector.
- Keep one-channel inference; stereo input is averaged to mono, wet output is copied to both channels, and dry channels retain their own aligned input.
- Support enhanced processing at 44.1, 48, 88.2, 96, 176.4, and 192 kHz. Invalid rates/layouts, excessive queue sizes, or startup failures initialize as unchanged direct bypass with zero reported latency.
- Preserve arbitrary-block continuity, one output per input sample, timestamp/generation matching, sample-time parameter smoothing, complete reset, and nonblocking aligned-dry fallback.
- Keep the 420 × 190 editor with only Attenuation Limit and Mix composite sliders. Numeric entry excludes the separate unit suffix; gestures use nice-plug's parameter setter. Preserve parameter ranges/defaults and the 50 ms attenuation ramp.
- Keep the project MIT license, required third-party/font notices, exact VST attribution, and embedded-model redistribution warning. Package both formats only through `scripts/package-release.sh`; preserve arm64/signature/ZIP/checksum checks, non-overwrite behavior, and `ditto --norsrc`. README image/audio assets stay out of the ZIP and `dist/` stays untracked.

## Current design

| Location | Responsibility |
| --- | --- |
| `plugin/src/lib.rs`, `params.rs` | nice-plug lifecycle, identity, parameters, Active/Bypass selection, latency reporting |
| `plugin/src/bridge.rs` | Preallocated host-block accumulation, stereo dry delay, Mix, timestamp/generation matching, mode-specific waiting |
| `plugin/src/worker.rs` | Persistent worker, bounded SPSC transport, startup/shutdown, reset and fault publication |
| `plugin/src/model.rs` | Worker-only one-channel `DfTract`, live metadata, pristine-model reconstruction |
| `plugin/src/resampler.rs`, `dsp.rs` | Persistent rubato conversion, model/raw path, checked latency calculation |
| `plugin/src/editor.rs` | GUI-only drawing, numeric entry, and host-synchronized parameter gestures |
| `plugin/Cargo.toml`, `Cargo.lock`, `xtask/` | Pinned dependencies, mutually exclusive model features, bundling |
| `scripts/package-release.sh` | Non-overwriting local release ZIP and SHA-256 sidecar |

`DfTract`, its pristine clone, model frames, and both converters stay on the worker. The callback must not allocate, lock, wait, log, run inference/resampling, reconstruct the model, or join threads. Initialization is transactional; failure selects direct bypass. Worker startup is bounded to ten seconds. Shutdown joins outside the callback only when finished within its two-second bound, otherwise detaches the isolated worker.

Chunks carry generation, starting sample, and length. Realtime/Buffered never wait; Offline may wait up to two seconds for a required chunk, with fault/stop escape and aligned dry on failure. Reset clears callback storage and smoothers in place, requests a new generation, and reconstructs model/resampler state on the worker. Overflow recovery instead preserves absolute host counters and the dry delay, restarts at the newest complete chunk, and masks wet output until the new history is valid.

Latency is fixed per initialization and uses the negotiated maximum block, not the current callback length:

```text
model_delay = fft_size - hop_size + lookahead * hop_size
core_delay_host = round((host_to_model_delay + model_delay) * host_rate / 48000)
                  + model_to_host_delay
runway_host = (ceil(max_buffer_size / host_quantum) + 1) * host_quantum
reported_latency = core_delay_host + runway_host
```

All arithmetic is checked. Dry and wet timelines include the same latency. At effectively zero attenuation, the worker continues advancing the model and selects an intrinsic-delay-aligned raw path. Model failure uses that aligned raw path; late worker output uses the bridge's aligned dry path.

## Acceptance evidence and its limits

The cleanup started from clean commit `d247b6b`. Source inspection and saved logs confirm the following completed repair evidence; these checks were not rerun for documentation cleanup.

| Unit | Recorded result and evidence |
| --- | --- |
| V10.1 | 31/31 tests passed in `/private/tmp/deepfilter-fix-tests.log`; execution 2 of 3, after an initial cache-permission denial |
| V10.2 | Allocation-asserting bundle and pluginval strictness 5 passed in `/private/tmp/deepfilter-fix-pluginval.log`; execution 1 of 3; nonfatal framework DPI/Arc warnings |
| V10.3 | Release build and asserted probe passed; `/private/tmp/deepfilter-audit-20260926/results-fixed.log` ends `REGRESSION_MATRIX_PASS`; execution 1 of 3. Packaging passed on execution 1 of 2 |
| Earlier build features | Standard-model compile and default feature inspection passed during the framework migration; this is historical evidence, not a fresh standard-model build of every later source revision |

At maximum block 1024, Rust impulse tests measured 2646/2400/4800 samples at 44.1/48/96 kHz, within one sample for conversion. They use zero attenuation to isolate alignment. The separate real-core reset test exercises nonzero attenuation. These are complementary checks.

The optimized probe observed 0% exact-dry substitution at paced 48 kHz blocks 128/512/1024/4096, transparent output by the 100 ms observation after a 100-to-0 attenuation change plus reported latency, and enhancement recovery within one second after forced overflow. Enhanced offline speech at 44.1/48/96 kHz was finite, non-silent, and identical after reset. Its suppressed enhanced-impulse residual is not latency evidence. The CLAP probe exercises shared DSP, not Resolve or the VST3 wrapper; pluginval supplies separate VST3 coverage.

Accepted executable SHA-256: `1a018b516090bfd434d423b03a667c763d362bd84e38c0c8e9253a70a6e55805`.
Package: `dist/DeepFilterNR-v0.6.0-fix-20260926-macos-arm64.zip` and `.zip.sha256`.
Archive SHA-256: `cd9a89641a086970dfeff7e07975e841360fe655a2dea3a0a46be7e87731230f`.
The approved installed VST3 is `~/Library/Audio/Plug-Ins/VST3/deepfilter-vst.vst3`; the previous bundle is retained at `dist/installed-backup-20260926/deepfilter-vst.vst3`. CLAP was packaged but not installed system-wide. Do not repeat installation on resume.

Temporary logs/probe files are supporting local evidence, not durable build prerequisites. The corrected host source is `/private/tmp/deepfilter-audit-20260926/probe-fixed.cpp`. Inspect its fixture/header dependencies before reuse. If unavailable for a future scheduling change, first define a bounded replacement check; do not silently claim a fake-worker test supplies equivalent timing evidence.

## Test review and proportional verification

All 31 current tests are retained. No whole test was established as redundant: helper boundary tests cover rejection/overflow paths, adapter tests prove bypass behavior, deterministic bridge tests control scheduling failures, and real-model tests cover actual inference and worker integration. Sharing assertions at different layers does not make those failure cases equivalent.

| Existing tests | Distinct coverage |
| --- | --- |
| `bridge.rs` (10) | Arbitrary partitions; mono/stereo Mix; late/fault/full fallback; overflow recovery; stale/future output; generation reset; mode wait policy; invalid shapes; real impulse alignment; real worker reset repeatability |
| `dsp.rs` (5) | Rate-domain latency; checked rounding; invalid/overflow geometry; maximum-block runway; enhanced model reset after nonzero audio |
| `resampler.rs` (5) | All supported quanta; invalid rates; model geometry; identity conversion/shape errors; checked integer helpers |
| `worker.rs` (3) | Chunk bounds/timestamps; reset/parameter publication; queue memory bounds |
| `lib.rs` (4), `editor.rs` (3), `model.rs` (1) | Direct bypass, sample-time attenuation, editor contract/input conversion, and live model shape/inference |

Removed from the execution workflow: repeated full-suite/pluginval phase gates, standalone compile checks immediately duplicated by tests, mandatory metadata scans on unrelated edits, obsolete agent assignments, and historical per-command approval bookkeeping. No new tests, test framework, coverage target, or temporary-host infrastructure is required for cleanup.

Choose only the applicable checks below for a future authorized change. Run commands from the repository root with the existing Rust toolchain. A passing test or bundle build already compiles its target; do not require an additional `cargo check` for that same target unless it resolves an earlier diagnostic.

| Changed area | Minimum sufficient check and expected result | Prerequisite / limit |
| --- | --- | --- |
| Documentation only (I11/V11) | Inspect claims/links and `git diff --check`; no whitespace errors or unintended files | No Rust build, pluginval, release probe, or packaging; 1 minute |
| Test source only | `cargo test --locked -p deepfilter-vst --lib <filter>` for affected tests; all selected tests pass | Existing fixtures; 20 minutes; no production bundle rebuild |
| Production Rust | Relevant filtered library tests; use `cargo test --locked -p deepfilter-vst --lib` once when changes span DSP/lifecycle modules | Default LL; 20 minutes; retain real-model serialization |
| Framework, editor integration, lifecycle, or callback safety | `cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs`, then `/Applications/pluginval.app/Contents/MacOS/pluginval --strictness-level 5 --validate-in-process target/bundled/deepfilter-vst.vst3`; build succeeds and validator reports `SUCCESS` without allocation abort | Current debug bundle and installed pluginval; separate units, 20 minutes each; do not rerun passed tests because validation failed |
| Model features or shared model compatibility | `cargo check --locked -p deepfilter-vst --no-default-features --features model-standard`; exit 0. Inspect guards/feature wiring; use `cargo tree -e features -p deepfilter-vst` only if resolution is uncertain | Affected manifests/source; 10 minutes; negative-feature builds are unnecessary when guards suffice |
| Real-worker scheduling, attenuation delivery, or overflow recovery | Scoped paced-host observation using the corrected probe or a documented replacement; finite audio, <=1% steady dry substitution, transparent 0 dB after 50 ms plus one block and latency, recovery within one second, repeatable enhanced reset where affected | Current optimized artifact; 2-minute probe; deterministic workers alone do not establish deadlines |
| New release artifact needed | `cargo xtask bundle deepfilter-vst --release`; exit 0, both bundles created | Affected acceptance checks passed; 20 minutes; final optimized artifact before packaging/manual host evidence |
| Packaging/distribution content | `zsh -n scripts/package-release.sh` if script changed; `./scripts/package-release.sh <unused-version>` when a new archive is requested; exit 0 with arm64, signatures, ZIP and SHA-256 verified | Accepted release bundles; 2 minutes, at most 2 packaging executions; never overwrite earlier packages |

For new smoke/targeted-test units, allow one initial execution plus two reruns after focused repairs. Carry unfinished-unit counts across edits and interruptions; do not rename a failure to reset its budget. A check prevented from launching by permission/tool setup is an environment issue, not behavioral evidence; obtain required tool permission and rerun that blocked stage. Keep logs outside the tracker and retrieve concise diagnostics. Stop when affected acceptance passes; do not broaden checks for reassurance.

Completed historical budgets remain closed: the refined-editor final build passed on attempt 4/4; V10 counters are recorded above. They are not lifetime limits on future authorized changes. Define affected units for a new request before execution. Routine transitions into authorized testing require a progress update, not renewed approval.

Production source, manifest, or build-configuration changes invalidate affected binary and manual-host evidence. Test-only changes invalidate affected test evidence; documentation-only changes do not invalidate binaries. Package-document changes require a new package only when producing a new distribution. Preserve still-relevant passing evidence and rerun only what changed. Optional external Steinberg validation is unconfigured and is not an outstanding requirement.

## Remaining work and dependency order

| ID | Outcome/location and action | Dependencies | Acceptance |
| --- | --- | --- | --- |
| I11 | Revise `PLANS.md` in place and align the verification paragraph in `AGENTS.md`; preserve product requirements, completed evidence, and deferred SC-11 | P11 | One tracker; current latency/recovery design; no repeated completed-work gates |
| V11 | Review the final documentation diff; leave source/tests/artifacts unchanged | I11 | `git diff --check` exits 0 and only the two intended Markdown files differ |
| SC-11 | Record final-artifact Resolve 21 playback/Deliver behavior | Accepted current release and available host/project; authorization for any new host mutations | Manual flow below |

SC-11 remains unverified, not waived. One earlier user-confirmed Deliver predates the final GUI/repair artifact and cannot close it. It is separate from the completed automated repair and this cleanup.

When host verification is undertaken, identify the bundle hash, project rates, and reported latency. At 48 kHz check mono/stereo playback, parameter changes, bypass, stop/play, and seek; render the same short section twice and compare non-silence, duration, and output content/checksum. At either 44.1 or 96 kHz check playback and one Deliver. Compare host latency with the impulse result for the same negotiated maximum block. Perform one bounded flow; if something fails, record that observation and the next targeted diagnostic instead of restarting the matrix. Do not impose a fixed wall-clock limit on user interaction.

The plan grants no new external-write permission. Reuse authorization already established for the current action; otherwise prepare the concrete result before requesting permission for installation, Resolve mutation, commit/push, or publication. Retain the model redistribution warning and do not claim legal clearance. This issue does not block local maintenance or testing.
