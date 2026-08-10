# DeepFilterNet3-VST3 Revision Instructions

## nice-plug + DeepFilterNet3-LL / Apple Silicon Mac

Revise this repository into a VST3 noise reduction plugin that not only appears to load correctly, but actually operates stably in both real-time processing and offline rendering.

Review the existing code and dependency library APIs, and use your own judgment to determine the optimal implementation approach.

---

## 0. Switch the Model to DeepFilterNet3-LL

### What is the Problem

The standard `DeepFilterNet3_onnx.tar.gz` used until now is a legitimate DeepFilterNet3 model, so it is not wrong per se.

However, for VST3 real-time use, additional latency is introduced due to the model's lookahead.

Furthermore, switching to externally found DeepFilterNet3 safetensors, MLX, or custom ONNX exports would break compatibility with the current `deep_filter` / `DfTract` inference pipeline, requiring a full reimplementation of the inference engine — separate from the primary VST3/DSP fixes.

The official DeepFilterNet v0.5.6 includes, alongside the standard model, an official Low-Latency model `DeepFilterNet3_ll_onnx.tar.gz` and a `default-model-ll` feature from the start.

### What to Do

Change the initial implementation target model to the official `DeepFilterNet3_ll_onnx.tar.gz`. [https://github.com/Rikorose/DeepFilterNet/tree/main/models]

Base `deep_filter` on the currently used v0.5.6 series, and use `default-model-ll` instead of the standard `default-model`.

For builds using LL, configure the build so that the standard model is not also embedded if it is not needed.

Do not convert the model to another format or integrate Hugging Face safetensors or custom ONNX exports.

However, the standard `DeepFilterNet3_onnx.tar.gz` is not a deletion candidate — keep it for comparison and fallback purposes. It should be selectable at build time and not selectable within the finished plugin.

### Why

The LL model is an official model that `DfParams` can load directly, so changes can be made while keeping the current tract/libDF pipeline intact.

The official real-time LADSPA plugin also uses a model without lookahead, targeting a minimum latency of 20 ms derived from the STFT.

However, Low-Latency is not synonymous with Low-CPU.

---

## 1. Fully Migrate from nih-plug to nice-plug

### What is the Problem

The current project is built assuming nih-plug, but the policy for this revision is to migrate to nice-plug. [https://codeberg.org/RustAudio/nice-plug]

Partially replacing the framework and leaving nih-plug and nice-plug dependencies, macros, xtask, and build procedures mixed together will introduce problems at distribution and host load time, even if it appears to work.

### What to Do

Unify everything under nice-plug, including the plugin body, export, parameters, xtask, and the bundle procedure. [https://codeberg.org/RustAudio/nice-plug]

Do not leave nih-plug-specific dependencies or code that has become unnecessary.

Do not change existing VST3 IDs, plugin names, parameter IDs, or anything else related to host-side compatibility without reason.

Ensure a state where a native Apple Silicon release VST3 bundle can be built.

### Why

The goal here is not the framework migration itself, but to properly implement DeepFilterNet3 as a VST3 plugin in a configuration that is easy to maintain long-term.

The more a half-baked compatibility layer is left in place, the more failure points are introduced that are unrelated to DSP.

---

## 2. Always Process DeepFilterNet with 1 Channel

### What is the Problem

The current code initializes the model with multiple channels for stereo input, while in practice only passing a mono-mixed input to the model.

The channel count the model expects does not match the actual tensor contents.

### What to Do

Keep the DeepFilterNet inference itself always at 1 channel.

Properly mix stereo input down to mono, pass it through a single DeepFilterNet3 instance, and distribute the processed result back to left and right. This is not a problem given that the primary use case for this plugin is "podcast post-processing and gaming/streaming vocal audio."

Always ensure the number of channels inside the model matches the number of channels in the frames actually passed to it.

### Why

The current structure risks allowing non-existent channel 2 data to influence mask calculations, leading to excessive attenuation or unstable results.

For noise reduction use, unifying to a single inference instance makes behavior and load more predictable.

---

## 3. Calculate the Actual Latency and Report It to the VST3 Host

### What is the Problem

Even though the model and STFT introduce delays, failing to correctly report the latency to the host will cause synchronization to break with other tracks.

Furthermore, directly mixing an undelayed dry signal with a delayed wet signal in a Dry/Wet Mix causes phase interference rather than a simple volume ratio.

Even though it is the LL model, latency must not be treated as zero.

### What to Do

Integrate the LL model into the actual processing pipeline and derive the real latency including the STFT, internal buffers, and frame processing.

Pass that value correctly to the VST3 / nice-plug latency reporting mechanism.

Additionally, measure it empirically using an impulse or similar, and confirm that the reported value matches the actual output position.

Apply the same delay to the dry side of the Dry/Wet Mix as to the wet side.

Do not hard-code 20 ms — the model's minimum value — without justification.

### Why

A host can only compensate if the plugin reports an accurate delay.

Also, the Mix parameter cannot function correctly unless the dry and wet signals are aligned on the same time axis.

---

## 4. Decouple the Host Block Size from DeepFilterNet's Hop Size

### What is the Problem

DeepFilterNet processes in fixed hop units, but the block size delivered from a VST3 host is not fixed.

Treating host blocks and model frames as equivalent causes input starvation, output starvation, dry leakage at the start, double playback, silence, and block-size-dependent latency.

### What to Do

Implement streaming processing that accumulates arbitrary-length audio from the host and converts it to the hop units required by DeepFilterNet.

Manage the output side with an independent queue or ring buffer as well.

Regardless of where block boundaries fall, maintain a structure where, on the time axis, one input sample corresponds to one output sample continuously.

Also explicitly manage the initial delay, and prohibit processing where dry audio accidentally leaks out due to insufficient buffer data, or where nothing is written and silence results.

### Why

The host-side block size and the neural network's frame size are separate entities.

By separating the two with a buffer layer, the same DSP path can be used for both real-time playback and offline rendering.

---

## 5. Make reset a True State Reset

### What is the Problem

Simply clearing a Vec or an outer queue does not return DeepFilterNet's internal state to its initial state.

If the STFT/ISTFT overlap state, model rolling state, normalization state, input/output queues, and dry delay are left over, the previous audio state will be carried into stop/play, seek, and render start operations.

### What to Do

On reset, return everything to a logically complete initial state — including the model, STFT-related state, normalization state, input buffer, output buffer, dry delay, and if a resampler is used, its streaming state as well.

However, design it so that unnecessary memory allocation does not occur within the audio thread.

Check the deep_filter and nice-plug implementations to determine which APIs to use for a safe reset.

### Why

DAWs perform stop, play, seek, and real-time to offline switching as routine operations.

A plugin that retains previous internal state may work in unit tests but will not be stable in a DAW.

---

## 6. Do Not Produce Silence at Sample Rates Other Than 48 kHz

### What is the Problem

DeepFilterNet3 itself assumes 48 kHz.

However, failing initialization simply because the host's sample rate is not 48 kHz can, depending on the host, result in the worst possible situation: "the plugin is inserted, but no audio comes out."

In previous implementations, this was a leading cause of the DaVinci Resolve offline rendering problem.

### What to Do

The ultimate goal is to stream-resample the input from the host rate to 48 kHz, process it with DeepFilterNet, and resample it back to the original host rate.

Do not recreate the resampler block by block — maintain its state as a continuous stream.

Include resampler-induced delay in the latency report.

Even if safe real-time resampling cannot be completed in this revision, do not cause the plugin initialization itself to fail and produce silence for sample rates other than 48 kHz.

In that case, fall back to a safe bypass and pass the input directly to the output.

### Why

"Not performing noise reduction under those conditions" and "eliminating the audio itself" are completely different things.

As a VST3 plugin, the latter is unacceptable.

---

## 7. Make process Real-Time Safe

### What is the Problem

The current state includes locks, dynamic memory allocation, Vec expansion, drain-based moves, and inefficient sample traversal within the process function.

Even if average CPU usage is low, this causes instantaneous audio dropouts.

### What to Do

Remove Mutex and unnecessary locks from the process hot path.

Allocate model frames, input/output buffers, dry delay, and if needed, resampler buffers, during the initialize phase.

Use fixed-capacity ring buffers or similar structures to avoid per-frame heap allocation and excessive memmove operations.

Ensure sample access is linear in time.

Only apply parameter values to the model when necessary, and avoid unnecessary log generation during process.

If nice-plug has a mechanism for detecting allocation during real-time, utilize it during development.

### Why

In the audio thread, worst-case stop time is the concern — not average performance.

Lock waits and the allocator must not be brought into the hot path of real-time DSP.

---

## 8. Do Not Use Separate Implementations for Real-Time Playback and Offline Rendering

### What is the Problem

Even if real-time playback in a DAW succeeds, if Deliver / Bounce / Export produces different results or silence, the plugin is not complete.

Adding separate ad-hoc processing only for offline rendering further complicates state management.

### What to Do

Pass both real-time and offline processing through the same DSP pipeline.

Design it to work regardless of changes in block size, processing speed, or reset timing from the host.

In particular, confirm that both normal playback and Deliver in DaVinci Resolve 20 work correctly.

Also confirm that state carry-over does not change results across stop→play, seek, multiple renders, and plugin enable/disable toggling.

### Why

DeepFilterNet itself is capable of streaming processing.

The place to absorb the difference between real-time and offline is not the model — it is the bridge layer between the model and the host.

---

## 9. Update README and License Information to Match the Implementation

### What is the Problem

Even if the code is fixed, if the README remains in an outdated state — referencing the old nih-plug, 48 kHz only, unusable in Resolve, etc. — the implementation and the documentation will be inconsistent.

Also, the licenses for the model, framework, and VST3-related dependencies each need to be verified against the actual dependencies.

### What to Do

Update the README, build instructions, supported sample rates, latency, model used, and known limitations to reflect the final code. Use proper README writing skills (screenshots are not required).

Verify the licenses of all dependencies that actually end up in the final binary, including the DeepFilterNet core and model, nice-plug, and VST3-related crates.

Do not write licenses based on guesswork.

Prepare the necessary LICENSE / NOTICE / third-party notices.

### Why

It is not the license written in the README, but the actual source and dependency tree that determines the distribution terms.

I plan to make the source public and release the VST3 plugin on GitHub.

---

# Work Policy

Extensive rewrites of existing code are permitted where necessary.

However, retraining DeepFilterNet itself, custom model conversion, and introduction of new inference backends are out of scope for this revision.

The official DeepFilterNet LADSPA implementation should be used as an important reference implementation when making decisions about real-time processing, buffering, and state management. [https://github.com/Rikorose/DeepFilterNet]