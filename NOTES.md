# Notes

## 2026-08-12

- The current manual-validation target is DaVinci Resolve 21; Resolve 20 is an
  older version retained only in the original requirements history.
- The user confirmed that DaVinci Resolve 21 completed a successful Deliver
  export with the plug-in applied.
- Resolve 21 displayed no plug-in UI. This is expected because the plug-in
  exposes host parameters but supplies no custom editor. The README screenshot
  shows Audacity's host-generated parameter interface.
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
