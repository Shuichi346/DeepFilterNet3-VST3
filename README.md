# DeepFilterNet3 VST3

DeepFilterNet3 VST3 is a macOS audio plugin that embeds the official DeepFilterNet v0.5.6 model for real-time and offline noise reduction. It exports VST3 and CLAP bundles through nice-plug, accepts mono or stereo tracks, and keeps neural inference and sample-rate conversion on a persistent worker so the host audio callback remains nonblocking.

## Features

- Official DeepFilterNet3 low-latency model by default, with the official standard model available as a separate build-time option.
- Mono and stereo input/output layouts with one mono inference stream.
- Arbitrary host block sizes through fixed, timestamped worker chunks.
- Streaming conversion for 44.1, 48, 88.2, 96, 176.4, and 192 kHz host rates.
- Reported latency with sample-aligned dry/wet mixing.
- The same DSP, resamplers, timeline, and reset protocol in Realtime, Buffered, and Offline modes.
- Lock-free callback transport and a latency-aligned dry fallback when a worker result is late.
- VST3 and CLAP exports with stable plugin and parameter IDs.

## Current validation scope

The current implementation is built and tested on Apple Silicon with macOS 26. Automated validation includes 24 Rust tests and pluginval strictness 5 with callback allocation assertions. pluginval exercised 44.1, 48, and 96 kHz processing and automation and completed with `SUCCESS`.

DaVinci Resolve 20 is the intended host, but the final interactive playback and Deliver smoke test has not yet been completed. Windows, Linux, and Intel macOS builds have not been validated.

## Audio behavior

The embedded model always receives one channel:

- Mono input is passed directly to inference.
- Stereo input is downmixed as `(left + right) / 2` for inference.
- The mono wet result is copied to both stereo outputs.
- Each stereo channel retains its own dry signal before the aligned dry/wet mix.

The plugin delays both dry and wet output to the reported latency. During startup or a real-time worker underrun, the affected samples use dry audio from the same delayed timestamp instead of silence or a stale wet frame. Offline mode uses the same worker pipeline and may wait up to two seconds for the required timestamped result.

Unsupported sample-rate or host-buffer geometry, model startup failure, and other initialization failures select unchanged direct bypass with zero reported latency.

## Latency

Latency is calculated from live model metadata, both resamplers, and two host quanta reserved for nonblocking collection and inference. The official low-latency model reports a 48 kHz FFT size of 960, hop size of 480, zero lookahead, and 480 samples of intrinsic model delay.

| Host rate | Host quantum | Reported latency | Impulse validation |
| ---: | ---: | ---: | :--- |
| 44.1 kHz | 441 samples | 1,764 samples (40 ms) | Within 1 sample |
| 48 kHz | 480 samples | 1,440 samples (30 ms) | Exact |
| 96 kHz | 960 samples | 3,840 samples (40 ms) | Within 1 sample |

Mix values of 0%, 50%, and 100% remain peak-aligned at the reported latency. The other declared rates use the same checked formula and streaming converter geometry.

## Requirements

- Apple Silicon Mac running macOS 26.x or later for the validated configuration.
- Rust 1.87 or later to build nice-plug 0.2.3.
- A VST3- or CLAP-compatible host.

The build downloads Rust dependencies and the pinned official DeepFilterNet v0.5.6 source/model archive.

## Build

Clone the repository and build the default low-latency model:

```bash
git clone https://github.com/Shuichi346/DeepFilterNet3-VST3.git
cd DeepFilterNet3-VST3
cargo xtask bundle deepfilter-vst --release
```

Generated bundles:

```text
target/bundled/deepfilter-vst.vst3
target/bundled/deepfilter-vst.clap
```

To build the official standard model instead of the default low-latency model:

```bash
cargo xtask bundle deepfilter-vst --release --no-default-features --features model-standard
```

The model features are mutually exclusive. Exactly one of `model-ll` or `model-standard` must be enabled.

## Install

For a user-only VST3 installation on macOS:

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/VST3"
cp -R target/bundled/deepfilter-vst.vst3 "$HOME/Library/Audio/Plug-Ins/VST3/"
```

For CLAP hosts:

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/CLAP"
cp -R target/bundled/deepfilter-vst.clap "$HOME/Library/Audio/Plug-Ins/CLAP/"
```

Restart or rescan the host after installation. Local builds are not distributed with a Developer ID signature or Apple notarization.

## Parameters

| Parameter | Range | Default | Behavior |
| :--- | ---: | ---: | :--- |
| Attenuation Limit | 0–100 dB | 100 dB | Limits the attenuation applied by DeepFilterNet. At effectively 0 dB, the model still advances while the aligned raw path is selected. |
| Mix | 0–100% | 100% | Blends latency-aligned per-channel dry audio with the mono wet result. |

## Development validation

Build a debug bundle with nice-plug's callback allocation assertions:

```bash
cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs
```

Run the bounded library and plugin validation gate:

```bash
cargo test -p deepfilter-vst --lib && \
/Applications/pluginval.app/Contents/MacOS/pluginval \
  --strictness-level 5 \
  --validate-in-process \
  target/bundled/deepfilter-vst.vst3
```

The VST3 bundle used for pluginval should be the allocation-asserting debug artifact from the preceding command.

## Known limitations

- The wet path is mono by design; stereo spatial differences remain only in the dry contribution.
- Real-time scheduling delays can temporarily substitute aligned dry audio for missing enhanced output.
- Worker/model startup is bounded to ten seconds; a startup failure selects direct bypass.
- Unsupported rate or buffer configurations select direct bypass rather than resampling approximately.
- DaVinci Resolve playback and Deliver behavior still require the final manual smoke check.
- Only the Apple Silicon macOS configuration described above has been validated.

## License

DeepFilterNet3-VST3 is available under the [MIT License](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option. See
[Third-Party Notices](THIRD_PARTY_NOTICES.md) for bundled models, libraries,
and VST 3 attribution.

## Credits

- [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) by Hendrik Schröter and contributors.
- [nice-plug](https://codeberg.org/RustAudio/nice-plug) by RustAudio contributors.
- [rubato](https://github.com/HEnquist/rubato) for streaming sample-rate conversion.
