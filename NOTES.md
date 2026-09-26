# Notes

## 2026-09-26

- Upgraded nice-plug/nice-plug-egui together to 0.4.2/0.5.1 and egui to
  0.36.2, with rtrb 0.3.5 and log 0.4.34 maintenance updates. The framework
  uses `activate`/`ActivateContext` and a concrete editor type; the GUI uses
  `NiceEguiApp` with host-driven repaint notifications and GUI-only parameter
  setters. GUI build resets temporary editing state when the window reopens.
- Selected rtrb 0.3.5 as the compatible maintenance update rather than the
  breaking 0.4 series. The worker continued to use individual push/pop operations.
- Retained DeepFilterNet 0.5.6, ndarray 0.15.6, Tract 0.19.16, and rubato
  0.14.1 to preserve the existing inference/resampling compatibility boundary.
  Rust 1.95 is required by the updated egui dependency; validation used 1.98.1.
- Confirmed all four embedded font license texts were unchanged in
  epaint_default_fonts 0.36.2. Updated harfrust attribution and added try-lock's
  MIT notice; removed the no-longer-resolved memoffset entry.
- All 31 library tests and allocation-asserting pluginval strictness 5 passed
  after the upgrade. Optimized VST3/CLAP bundles were packaged with verified
  arm64 architecture, ad-hoc signatures, ZIP integrity, and SHA-256 output.
- Dependency-upgrade acceptance and artifact provenance are tracked under
  I13/V13 in `PLANS.md`. The plugin remained version 0.7.0; the package used
  the `0.7.0-deps-20260926` suffix to preserve the previous archive. This build
  was neither installed nor published. Existing installed binaries, paced-host
  probe measurements, and Resolve evidence predated this migration.

- Updated the plugin from 0.6.0 to 0.7.0, rebuilt both formats, and confirmed
  the actual exported plugin descriptor reports 0.7.0. The bundler's generic
  Info.plist version is independent of the host-facing plugin VERSION.
- Created the verified v0.7.0 ZIP and checksum sidecar, and installed its VST3
  with the previous installed bundle preserved in `dist/installed-backup-before-v070/`.
  DSP and dependencies were unchanged, so the existing behavioral evidence was reused.

- Plan maintenance removed duplicate trackers, superseded instructions, and
  repeated acceptance gates. `PLANS.md` retained current design and acceptance
  evidence; historical construction details remained in Git and these notes.
- Review retained all 31 Rust tests because they covered distinct failure
  cases. The cleanup changed documentation only; existing binary evidence
  remained applicable, so no rebuild or code-test rerun was needed.
- The operational audit reproduced callback-counted attenuation smoothing,
  dry substitution at 1024/4096-sample callbacks, and latched dry-only output
  after input overflow. Historical pluginval success did not detect these.
- Attenuation now advances the existing smoother by the callback's sample
  count. Parameter automation remains block-based; the nominal ramp is 50 ms.
- Worker runway now covers the negotiated maximum callback rounded up to a
  quantum, plus one inference quantum. At 48 kHz, maxima of 128/512/1024/4096
  samples report 1440/1920/2400/5280 samples of total latency respectively.
- Overflow recovery starts a new generation at the newest complete chunk,
  preserves absolute host counters and the stereo dry ring, and masks reset
  model/resampler history with aligned dry until it is valid.
- All 31 library tests passed, including new parameter-duration, runway, and
  recovery regressions. At maximum block 1024, measured impulse latency was
  2646/2400/4800 samples at 44.1/48/96 kHz, with the declared one-sample
  tolerance for conversion. Release/host evidence is tracked in PLANS.md.
- Allocation-asserting pluginval strictness 5 completed with SUCCESS. The
  optimized release probe measured zero dry substitutions at paced 48 kHz
  blocks of 128/512/1024/4096, a transparent 0 dB path by 100 ms after the
  change plus reported latency, and enhancement recovery within one second
  after a forced queue overrun. Enhanced offline speech at 44.1/48/96 kHz
  remained non-silent and bit-identical after reset. These are bounded host
  measurements, not a guarantee against arbitrary OS scheduling stalls.
- The verified release was packaged as `dist/DeepFilterNR-v0.6.0-fix-20260926-macos-arm64.zip`
  with its SHA-256 sidecar. With separate installation approval, the user VST3
  was replaced and the previous bundle retained under
  `dist/installed-backup-20260926/`. Running hosts must reload to use the new binary.

## 2026-08-12

- The plug-in now defines a fixed 420 × 190 logical-pixel custom editor using
  nice-plug-egui 0.3.0 and egui 0.35.0. Its only controls are English
  Attenuation Limit and Mix parameter sliders.
- The custom editor uses nice-plug's parameter-aware slider gestures and does
  not add an audio-to-GUI channel, model selector, waveform, or meter.
- The GUI dependency embeds default fonts, so the repository and packaging
  inventory now retain their upstream font notices and license texts.
- All 25 library tests passed after the editor addition. pluginval strictness
  5 opened the editor idle and during processing, exercised editor automation
  and the existing multi-rate DSP matrix, and reported `SUCCESS`.
- pluginval emitted non-fatal nice-plug warnings when it requested an explicit
  macOS DPI scale; nice-plug used system scaling and the editor tests passed.
- The current manual-validation target is DaVinci Resolve 21; Resolve 20 is an
  older version retained only in the original requirements history.
- The user confirmed that DaVinci Resolve 21 completed a successful Deliver
  export with the plug-in applied.
- Resolve 21 displayed no plug-in UI for the earlier bundle because that
  artifact supplied no custom editor. The README screenshot now shows the
  refined custom editor with unit-separated numeric entry.
- The successful Deliver is partial SC-11 evidence. The sample rate, bundle
  hash, host latency, repeat-render measurements, playback interactions, and
  non-48 kHz case were not recorded, so the full host matrix remains open.

## 2026-08-11

- The project and plug-in release version was changed from 0.1.0 to 0.5.0;
  the private `xtask` helper retained its independent 0.1.0 package version.
- `scripts/package-release.sh` produced the verified 0.5.0 Apple Silicon
  package at `dist/DeepFilterNR-v0.5.0-macos-arm64.zip`; its SHA-256 is
  `b50c4e97073743cc91c905a04e9c349de4bd96fc181f8f3d3dcae34d4fb43204`.
- License documentation was reduced to required Apache-2.0, MIT, ISC, Unicode,
  and VST notices plus the unresolved model-redistribution warning. The
  transitive license-expression table and dependency-purpose prose were
  removed.
- The documentation-only package regeneration preserved both VST3 and CLAP
  binary hashes at
  `6b9e074022a3db8cd5ffcf01d1b2fc49943d51e7a3735195d42b6a8b6c4d8e56`;
  ZIP integrity, sidecar verification, and inventory inspection passed.
- All 24 Rust library tests passed, including real-model reset and latency
  checks. Measured latency was 1764 samples at 44.1 kHz, 1440 at 48 kHz, and
  3840 at 96 kHz.
- pluginval strictness 5 passed the allocation-asserting debug VST3 across
  44.1, 48, and 96 kHz processing, state, automation, editor, parameter, and
  bus checks.
- `cargo xtask bundle deepfilter-vst --release` passed on final attempt 2/3
  after the MIT manifest change and recreated release VST3 and CLAP bundles
  under `target/bundled/`.
- The final release build emitted only private-visibility and unused-helper
  warnings. No build error remained.
- The earlier packaging run produced the superseded 0.1.0 candidate at
  `dist/DeepFilterNR-v0.1.0-macos-arm64.zip`; its accepted SHA-256 is
  `5a84c441835bbeefa69c20a301e9c07b3e99a5fc5821b3fa1d35fadb12a36ce8`.
- The first package inventory exposed unwanted `._*` AppleDouble entries.
  Adding `ditto --norsrc` removed them; the accepted archive passed ZIP and
  SHA-256 verification and contains no repository screenshot or WAV assets.
- The project license was changed to MIT. Canonical Apache-2.0 and Unicode v3
  texts were retained under `third-party-licenses/` only for applicable
  third-party components.
- DeepFilterNet issue #697 still had no answer confirming redistribution terms
  for pretrained model archives, so the binary package was prepared locally
  but not published.
- The original requirements targeted DaVinci Resolve 20, which was the older
  host version at the time of planning. That test was deferred and was later
  superseded by the Resolve 21 user evidence recorded above.
- Cargo registry sandbox denials were avoided by granting the Cargo commands
  elevated cache access. No persistent Codex setting change was required.
