# Notes

## 2026-08-11

- All 24 Rust library tests passed, including real-model reset and latency
  checks. Measured latency was 1764 samples at 44.1 kHz, 1440 at 48 kHz, and
  3840 at 96 kHz.
- pluginval strictness 5 passed the allocation-asserting debug VST3 across
  44.1, 48, and 96 kHz processing, state, automation, editor, parameter, and
  bus checks.
- `cargo xtask bundle deepfilter-vst --release` passed on final attempt 1/3
  and created release VST3 and CLAP bundles under `target/bundled/`.
- The final release build emitted only private-visibility and unused-helper
  warnings. No build error remained.
- The DaVinci Resolve 20 playback and Deliver smoke test was deferred by the
  user. `PLANS.md` leaves SC-11 open; Codex did not install the bundle or
  operate Resolve.
- Cargo registry sandbox denials were avoided by granting the Cargo commands
  elevated cache access. No persistent Codex setting change was required.
