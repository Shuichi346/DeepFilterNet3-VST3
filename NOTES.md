# Notes

## 2026-08-11

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
- `scripts/package-release.sh` produced the 80 MB Apple Silicon candidate at
  `dist/DeepFilterNR-v0.1.0-macos-arm64.zip`; its accepted SHA-256 is
  `5a84c441835bbeefa69c20a301e9c07b3e99a5fc5821b3fa1d35fadb12a36ce8`.
- The first package inventory exposed unwanted `._*` AppleDouble entries.
  Adding `ditto --norsrc` removed them; the accepted archive passed ZIP and
  SHA-256 verification and contains no repository screenshot or WAV assets.
- The project license was changed to MIT. Apache-2.0 text was retained under
  `third-party-licenses/` only for applicable third-party components.
- DeepFilterNet issue #697 still had no answer confirming redistribution terms
  for pretrained model archives, so the binary package was prepared locally
  but not published.
- The DaVinci Resolve 20 playback and Deliver smoke test was deferred by the
  user. `PLANS.md` leaves SC-11 open; Codex did not install the bundle or
  operate Resolve.
- Cargo registry sandbox denials were avoided by granting the Cargo commands
  elevated cache access. No persistent Codex setting change was required.
