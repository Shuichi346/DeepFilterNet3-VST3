# Third-Party Notices

This file records the principal third-party software and model assets used to
build DeepFilterNet3-VST3. The exact resolved Rust package versions are recorded
in `Cargo.lock`. License links below point to the corresponding upstream
projects; this notice does not replace their license texts or terms.

## Runtime and bundled components

### DeepFilterNet / `deep_filter` 0.5.6

- Source: <https://github.com/Rikorose/DeepFilterNet/tree/v0.5.6>
- License: MIT OR Apache-2.0, at the recipient's option
- Copyright: Hendrik Schröter and DeepFilterNet contributors

The plugin builds the `deep_filter` Rust library from the official v0.5.6 tag.
Its repository provides `LICENSE-MIT` and `LICENSE-APACHE`.

The default `model-ll` feature embeds the official
`models/DeepFilterNet3_ll_onnx.tar.gz` archive from that tag. The optional
`model-standard` feature embeds the official
`models/DeepFilterNet3_onnx.tar.gz` archive from the same tag. No separate
model-weight license file was present beside those archives or within either
archive in the inspected v0.5.6 source tree. This records the upstream facts
and does not infer terms beyond the licenses published by that repository.

### nice-plug framework and bundler

- Source: <https://codeberg.org/RustAudio/nice-plug>
- License: ISC
- Authors: Robbert van der Helm, Billy Messenger, and contributors

The resolved ISC-licensed packages are `nice-plug` 0.2.3,
`nice-plug-core` 0.2.0, `nice-plug-derive` 0.1.2, and `nice-log` 0.3.1.
`nice-plug-xtask` 0.1.1 is used only by the repository's bundle command.

### rubato 0.14.1

- Source: <https://github.com/HEnquist/rubato/tree/v0.14.1>
- License: MIT
- Copyright: Hendrik Enquist and contributors

rubato provides the fixed-size sample-rate converters.

### rtrb 0.3.3

- Source: <https://github.com/mgeier/rtrb/tree/v0.3.3>
- License: MIT OR Apache-2.0, at the recipient's option

rtrb provides the single-producer/single-consumer ring buffers used at the
audio-thread boundary.

### ndarray 0.15.6

- Source: <https://github.com/rust-ndarray/ndarray/tree/0.15.6>
- License: MIT OR Apache-2.0, at the recipient's option

### log 0.4.x

- Source: <https://github.com/rust-lang/log>
- License: MIT OR Apache-2.0, at the recipient's option

## Additional transitive license families

The locked default Apple Silicon dependency tree was compared with package
license metadata. Most remaining normal transitive Rust packages offer MIT,
Apache-2.0, or both. The following resolved packages have additional or more
specific terms and are called out so those terms are not hidden by that
summary:

| Package | Resolved license expression |
| :--- | :--- |
| [`tiny-keccak` 2.0.2](https://crates.io/crates/tiny-keccak/2.0.2) | CC0-1.0 |
| [`unicode-ident` 1.0.24](https://crates.io/crates/unicode-ident/1.0.24) | (MIT OR Apache-2.0) AND Unicode-3.0 |
| [`anymap3` 1.1.0](https://crates.io/crates/anymap3/1.1.0) | BlueOak-1.0.0 OR MIT OR Apache-2.0 |
| [`adler2` 2.0.1](https://crates.io/crates/adler2/2.0.1) | 0BSD OR MIT OR Apache-2.0 |
| [`zerocopy` and `zerocopy-derive` 0.8.56](https://crates.io/crates/zerocopy/0.8.56) | BSD-2-Clause OR Apache-2.0 OR MIT |
| [`fragile` 2.1.0](https://crates.io/crates/fragile/2.1.0) | Apache-2.0 |
| [`prost` and `prost-derive` 0.11.9](https://crates.io/crates/prost/0.11.9) | Apache-2.0 |
| [`dpi` 0.1.2](https://crates.io/crates/dpi/0.1.2) | Apache-2.0 AND MIT |
| [`atomic_float` 1.1.0](https://crates.io/crates/atomic_float/1.1.0) | Apache-2.0 OR MIT OR Unlicense |
| [`miniz_oxide` 0.8.9](https://crates.io/crates/miniz_oxide/0.8.9) | MIT OR Zlib OR Apache-2.0 |
| [`raw-window-handle` 0.6.2](https://crates.io/crates/raw-window-handle/0.6.2) | MIT OR Apache-2.0 OR Zlib |
| [`tinyvec` 1.12.0 and `tinyvec_macros` 0.1.1](https://crates.io/crates/tinyvec/1.12.0) | Zlib OR Apache-2.0 OR MIT |
| [`aho-corasick` 1.1.5](https://crates.io/crates/aho-corasick/1.1.5), [`byteorder` 1.5.0](https://crates.io/crates/byteorder/1.5.0), and [`memchr` 2.8.3](https://crates.io/crates/memchr/2.8.3) | Unlicense OR MIT |
| [`same-file` 1.0.6](https://crates.io/crates/same-file/1.0.6) and [`walkdir` 2.5.0](https://crates.io/crates/walkdir/2.5.0) | Unlicense OR MIT |

Where a package offers multiple licenses with `OR`, recipients may use it
under one of the offered licenses. An `AND` expression requires compliance
with each named license. The upstream package source remains authoritative.

## VST 3 interfaces

### `vst3` crate 0.3.0

- Source: <https://github.com/coupler-rs/vst3-rs/tree/v0.3.0>
- License: MIT OR Apache-2.0, at the recipient's option

The VST 3 Rust interface crate is resolved through nice-plug's VST 3 support.

### Steinberg VST 3 SDK

- Source and license: <https://github.com/steinbergmedia/vst3sdk>
- License: MIT for VST 3 SDK version 3.8 and later
- Copyright: Steinberg Media Technologies GmbH
- Licensing and trademark guidance:
  <https://steinbergmedia.github.io/vst3_dev_portal/pages/VST+3+Licensing/Index.html>

VST is a trademark of Steinberg Media Technologies GmbH. Use of the VST name
or logo must follow Steinberg's published trademark guidelines.
