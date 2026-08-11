//! Callback-owned host/worker bridge with timestamped wet/dry alignment.
//!
//! A worker result keeps the input stream index of its chunk. Two collected
//! host quanta form the runway, so host output index `t` consumes worker stream
//! index `t - 2 * host_quantum`; the worker DSP has already applied its own
//! intrinsic and resampler delay to that stream.

use std::time::{Duration, Instant};

use nice_plug::prelude::ProcessMode;
use rtrb::PopError;

use crate::worker::{AudioChunk, SubmitError, WorkerHandle, MAX_HOST_QUANTUM};

const RUNWAY_QUANTA: usize = 2;
const SAFETY_QUANTA: usize = 2;
const OFFLINE_OUTPUT_TIMEOUT: Duration = Duration::from_secs(2);
const OFFLINE_POLL_INTERVAL: Duration = Duration::from_micros(100);

/// Checked configuration needed to create the callback side of a worker.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BridgeConfig {
    pub(crate) channels: usize,
    pub(crate) max_buffer_size: u32,
    pub(crate) host_quantum: usize,
    pub(crate) reported_latency: u32,
    pub(crate) process_mode: ProcessMode,
    pub(crate) queue_capacity: usize,
}

/// Construction failures select the plugin's direct-bypass state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BridgeConfigError {
    InvalidChannelCount,
    ZeroMaximumBufferSize,
    InvalidHostQuantum,
    ArithmeticOverflow,
    QueueCapacityMismatch,
    WorkerQuantumMismatch,
    ReportedLatencyMismatch,
    LatencyShorterThanRunway,
    DryDelayTooLarge,
    DeferredInputTooLarge,
    WorkerUnavailable,
}

/// Callback-time input mismatch. The affected buffer is left as direct input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BridgeProcessError {
    ChannelMismatch,
    BlockTooLarge,
}

/// Atomically observed worker state useful to the framework lifecycle layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BridgeStatus {
    pub(crate) ready: bool,
    pub(crate) faulted: bool,
    pub(crate) model_degraded: bool,
    pub(crate) input_discontinuous: bool,
    pub(crate) stopped: bool,
    pub(crate) acknowledged_generation: u64,
}

#[derive(Clone, Copy, Default)]
struct DryFrame {
    left: f32,
    right: f32,
}

#[derive(Clone, Copy)]
struct OfflineWait {
    generation: u64,
    chunk_start: u64,
    deadline: Instant,
}

/// Narrow callback-side worker interface. Tests implement this with a fully
/// deterministic delayed-identity engine, while production uses `WorkerHandle`.
trait BridgeWorker {
    fn dsp_info(&self) -> crate::dsp::DspInfo;
    fn submit(&mut self, chunk: AudioChunk) -> Result<(), SubmitError>;
    fn pop_output(&mut self) -> Result<AudioChunk, PopError>;
    fn request_reset(&self, generation: u64);
    fn set_attenuation(&self, attenuation: f32);
    fn requested_generation(&self) -> u64;
    fn acknowledged_generation(&self) -> u64;
    fn is_ready(&self) -> bool;
    fn is_faulted(&self) -> bool;
    fn is_model_faulted(&self) -> bool;
    fn is_discontinuous(&self) -> bool;
    fn is_stopped(&self) -> bool;
    fn mark_faulted(&self);
    fn mark_discontinuous(&self);
    fn offline_wait(&mut self, interval: Duration);
    fn shutdown(&mut self);
}

impl BridgeWorker for WorkerHandle {
    fn dsp_info(&self) -> crate::dsp::DspInfo {
        WorkerHandle::dsp_info(self)
    }

    fn submit(&mut self, chunk: AudioChunk) -> Result<(), SubmitError> {
        WorkerHandle::submit(self, chunk)
    }

    fn pop_output(&mut self) -> Result<AudioChunk, PopError> {
        WorkerHandle::pop_output(self)
    }

    fn request_reset(&self, generation: u64) {
        WorkerHandle::request_reset(self, generation);
    }

    fn set_attenuation(&self, attenuation: f32) {
        WorkerHandle::set_attenuation(self, attenuation);
    }

    fn requested_generation(&self) -> u64 {
        WorkerHandle::requested_generation(self)
    }

    fn acknowledged_generation(&self) -> u64 {
        WorkerHandle::acknowledged_generation(self)
    }

    fn is_ready(&self) -> bool {
        WorkerHandle::is_ready(self)
    }

    fn is_faulted(&self) -> bool {
        WorkerHandle::is_faulted(self)
    }

    fn is_model_faulted(&self) -> bool {
        WorkerHandle::is_model_faulted(self)
    }

    fn is_discontinuous(&self) -> bool {
        WorkerHandle::is_discontinuous(self)
    }

    fn is_stopped(&self) -> bool {
        WorkerHandle::is_stopped(self)
    }

    fn mark_faulted(&self) {
        WorkerHandle::mark_faulted(self);
    }

    fn mark_discontinuous(&self) {
        WorkerHandle::mark_discontinuous(self);
    }

    fn offline_wait(&mut self, interval: Duration) {
        std::thread::sleep(interval);
    }

    fn shutdown(&mut self) {
        WorkerHandle::shutdown(self);
    }
}

/// Fixed-storage bridge between arbitrary host blocks and fixed worker chunks.
pub(crate) struct HostBridge<W = WorkerHandle> {
    worker: W,
    channels: usize,
    max_buffer_size: usize,
    host_quantum: usize,
    reported_latency: usize,
    collection_runway: usize,
    process_mode: ProcessMode,
    generation: u64,
    input_samples: u64,
    output_samples: u64,
    input_chunk_start: u64,
    input_len: usize,
    input_accumulation: [f32; MAX_HOST_QUANTUM],
    deferred_input: Box<[AudioChunk]>,
    deferred_read: usize,
    deferred_write: usize,
    deferred_len: usize,
    dry_delay: Box<[DryFrame]>,
    dry_cursor: usize,
    pending_output: Option<AudioChunk>,
    queue_scan_limit: usize,
    offline_wait: Option<OfflineWait>,
    offline_timed_out: Option<(u64, u64)>,
}

impl HostBridge<WorkerHandle> {
    /// Calculate the two SPSC capacities before starting the worker transaction.
    pub(crate) fn queue_capacity(
        max_buffer_size: u32,
        host_quantum: usize,
    ) -> Result<usize, BridgeConfigError> {
        if max_buffer_size == 0 {
            return Err(BridgeConfigError::ZeroMaximumBufferSize);
        }
        if host_quantum == 0 || host_quantum > MAX_HOST_QUANTUM {
            return Err(BridgeConfigError::InvalidHostQuantum);
        }

        let max_buffer_size = usize::try_from(max_buffer_size)
            .map_err(|_| BridgeConfigError::ArithmeticOverflow)?;
        let rounded = max_buffer_size
            .checked_add(host_quantum - 1)
            .ok_or(BridgeConfigError::ArithmeticOverflow)?;
        let block_chunks = rounded / host_quantum;
        block_chunks
            .checked_add(RUNWAY_QUANTA)
            .and_then(|capacity| capacity.checked_add(SAFETY_QUANTA))
            .filter(|capacity| *capacity != 0)
            .ok_or(BridgeConfigError::ArithmeticOverflow)
    }

    /// Finish a worker-startup transaction with callback-owned fixed storage.
    pub(crate) fn new(
        worker: WorkerHandle,
        config: BridgeConfig,
    ) -> Result<Self, BridgeConfigError> {
        Self::build(worker, config)
    }
}

impl<W: BridgeWorker> HostBridge<W> {
    fn build(worker: W, config: BridgeConfig) -> Result<Self, BridgeConfigError> {
        if config.channels != 1 && config.channels != 2 {
            return Err(BridgeConfigError::InvalidChannelCount);
        }
        if config.host_quantum == 0 || config.host_quantum > MAX_HOST_QUANTUM {
            return Err(BridgeConfigError::InvalidHostQuantum);
        }
        if config.max_buffer_size == 0 {
            return Err(BridgeConfigError::ZeroMaximumBufferSize);
        }
        if !worker.is_ready() || worker.is_faulted() || worker.is_stopped() {
            return Err(BridgeConfigError::WorkerUnavailable);
        }
        let dsp_info = worker.dsp_info();
        if dsp_info.host_quantum != config.host_quantum {
            return Err(BridgeConfigError::WorkerQuantumMismatch);
        }
        if dsp_info.latency.total_host != config.reported_latency {
            return Err(BridgeConfigError::ReportedLatencyMismatch);
        }

        let queue_capacity = HostBridge::<WorkerHandle>::queue_capacity(
            config.max_buffer_size,
            config.host_quantum,
        )?;
        if config.queue_capacity != queue_capacity {
            return Err(BridgeConfigError::QueueCapacityMismatch);
        }

        let collection_runway = config
            .host_quantum
            .checked_mul(RUNWAY_QUANTA)
            .ok_or(BridgeConfigError::ArithmeticOverflow)?;
        let reported_latency = usize::try_from(config.reported_latency)
            .map_err(|_| BridgeConfigError::ArithmeticOverflow)?;
        if reported_latency < collection_runway {
            return Err(BridgeConfigError::LatencyShorterThanRunway);
        }
        let dry_delay_len = reported_latency
            .checked_add(1)
            .ok_or(BridgeConfigError::DryDelayTooLarge)?;

        let mut dry_delay = Vec::new();
        dry_delay
            .try_reserve_exact(dry_delay_len)
            .map_err(|_| BridgeConfigError::DryDelayTooLarge)?;
        dry_delay.resize(dry_delay_len, DryFrame::default());

        let empty_chunk = AudioChunk::silence(0, 0, 0)
            .map_err(|_| BridgeConfigError::DeferredInputTooLarge)?;
        let mut deferred_input = Vec::new();
        deferred_input
            .try_reserve_exact(queue_capacity)
            .map_err(|_| BridgeConfigError::DeferredInputTooLarge)?;
        deferred_input.resize(queue_capacity, empty_chunk);

        Ok(Self {
            generation: worker.requested_generation(),
            worker,
            channels: config.channels,
            max_buffer_size: usize::try_from(config.max_buffer_size)
                .map_err(|_| BridgeConfigError::ArithmeticOverflow)?,
            host_quantum: config.host_quantum,
            reported_latency,
            collection_runway,
            process_mode: config.process_mode,
            input_samples: 0,
            output_samples: 0,
            input_chunk_start: 0,
            input_len: 0,
            input_accumulation: [0.0; MAX_HOST_QUANTUM],
            deferred_input: deferred_input.into_boxed_slice(),
            deferred_read: 0,
            deferred_write: 0,
            deferred_len: 0,
            dry_delay: dry_delay.into_boxed_slice(),
            dry_cursor: 0,
            pending_output: None,
            queue_scan_limit: queue_capacity,
            offline_wait: None,
            offline_timed_out: None,
        })
    }

    pub(crate) fn reported_latency(&self) -> u32 {
        match u32::try_from(self.reported_latency) {
            Ok(latency) => latency,
            Err(_) => u32::MAX,
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn status(&self) -> BridgeStatus {
        BridgeStatus {
            ready: self.worker.is_ready(),
            faulted: self.worker.is_faulted(),
            model_degraded: self.worker.is_model_faulted(),
            input_discontinuous: self.worker.is_discontinuous(),
            stopped: self.worker.is_stopped(),
            acknowledged_generation: self.worker.acknowledged_generation(),
        }
    }

    /// Publish attenuation without crossing the callback/worker ownership boundary.
    pub(crate) fn set_attenuation(&self, attenuation: f32) {
        self.worker.set_attenuation(attenuation);
    }

    /// Process one mono host block, calling `mix` exactly once per input sample.
    pub(crate) fn process_mono<M>(
        &mut self,
        samples: &mut [f32],
        mut mix: M,
    ) -> Result<(), BridgeProcessError>
    where
        M: FnMut() -> f32,
    {
        if self.channels != 1 {
            self.worker.mark_faulted();
            return Err(BridgeProcessError::ChannelMismatch);
        }
        if samples.len() > self.max_buffer_size {
            self.worker.mark_faulted();
            return Err(BridgeProcessError::BlockTooLarge);
        }

        for sample in samples {
            let original = *sample;
            let output = self.process_sample(original, original, original, mix());
            *sample = output.0;
        }
        Ok(())
    }

    /// Process one stereo host block, keeping each channel's dry path independent.
    pub(crate) fn process_stereo<M>(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        mut mix: M,
    ) -> Result<(), BridgeProcessError>
    where
        M: FnMut() -> f32,
    {
        if self.channels != 2 || left.len() != right.len() {
            self.worker.mark_faulted();
            return Err(BridgeProcessError::ChannelMismatch);
        }
        if left.len() > self.max_buffer_size {
            self.worker.mark_faulted();
            return Err(BridgeProcessError::BlockTooLarge);
        }

        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let original_left = *left;
            let original_right = *right;
            let inference = (original_left + original_right) * 0.5;
            let output = self.process_sample(original_left, original_right, inference, mix());
            *left = output.0;
            *right = output.1;
        }
        Ok(())
    }

    /// Reset only callback-owned storage, then publish the new generation atomically.
    pub(crate) fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.input_samples = 0;
        self.output_samples = 0;
        self.input_chunk_start = 0;
        self.input_len = 0;
        self.input_accumulation.fill(0.0);
        self.deferred_read = 0;
        self.deferred_write = 0;
        self.deferred_len = 0;
        self.dry_delay.fill(DryFrame::default());
        self.dry_cursor = 0;
        self.pending_output = None;
        self.offline_wait = None;
        self.offline_timed_out = None;
        self.worker.request_reset(self.generation);
    }

    /// Lifecycle-only worker shutdown. Never call this from `process_*` or `reset`.
    pub(crate) fn shutdown(&mut self) {
        self.worker.shutdown();
    }

    fn process_sample(
        &mut self,
        original_left: f32,
        original_right: f32,
        inference: f32,
        mix: f32,
    ) -> (f32, f32) {
        self.accumulate_input(inference);
        let dry = self.store_and_delay_dry(original_left, original_right);
        let output_time = self.output_samples;
        let wet = match self.process_mode {
            ProcessMode::Offline => self.wait_for_wet(output_time),
            ProcessMode::Realtime | ProcessMode::Buffered => self.poll_wet(output_time),
        };
        self.output_samples = self.output_samples.wrapping_add(1);

        let mix = sanitize_mix(mix);
        let wet_left = wet.unwrap_or(dry.left);
        let wet_right = wet.unwrap_or(dry.right);
        (
            dry.left * (1.0 - mix) + wet_left * mix,
            dry.right * (1.0 - mix) + wet_right * mix,
        )
    }

    fn accumulate_input(&mut self, sample: f32) {
        if self.input_len == 0 {
            self.input_chunk_start = self.input_samples;
        }
        if self.input_len >= self.host_quantum {
            self.worker.mark_faulted();
            self.input_len = 0;
        }
        let Some(slot) = self.input_accumulation.get_mut(self.input_len) else {
            self.worker.mark_faulted();
            self.input_len = 0;
            return;
        };
        *slot = sample;
        self.input_len += 1;
        self.input_samples = self.input_samples.wrapping_add(1);

        if self.input_len == self.host_quantum {
            match self.input_accumulation.get(..self.host_quantum) {
                Some(samples) => match AudioChunk::from_slice(
                    self.generation,
                    self.input_chunk_start,
                    samples,
                ) {
                    Ok(chunk) => self.submit_or_defer(chunk),
                    Err(_) => self.worker.mark_faulted(),
                },
                None => self.worker.mark_faulted(),
            }
            self.input_len = 0;
        }
    }

    fn submit_or_defer(&mut self, chunk: AudioChunk) {
        self.flush_deferred_input();
        if self.deferred_len != 0 {
            self.defer_input(chunk);
            return;
        }

        match self.worker.submit(chunk) {
            Ok(()) => {}
            Err(SubmitError::Full(chunk)) => self.defer_input(chunk),
            Err(SubmitError::Unavailable(chunk)) => {
                if !self.worker.is_faulted() && !self.worker.is_stopped() {
                    self.defer_input(chunk);
                }
            }
        }
    }

    fn flush_deferred_input(&mut self) {
        for _ in 0..self.queue_scan_limit {
            if self.deferred_len == 0 {
                return;
            }
            let Some(chunk) = self.deferred_input.get(self.deferred_read).cloned() else {
                self.worker.mark_faulted();
                return;
            };
            match self.worker.submit(chunk) {
                Ok(()) => {
                    self.deferred_read = (self.deferred_read + 1) % self.deferred_input.len();
                    self.deferred_len -= 1;
                }
                Err(SubmitError::Full(_)) | Err(SubmitError::Unavailable(_)) => return,
            }
        }
    }

    fn defer_input(&mut self, chunk: AudioChunk) {
        if self.deferred_len >= self.deferred_input.len() {
            self.worker.mark_discontinuous();
            return;
        }
        let Some(slot) = self.deferred_input.get_mut(self.deferred_write) else {
            self.worker.mark_faulted();
            return;
        };
        *slot = chunk;
        self.deferred_write = (self.deferred_write + 1) % self.deferred_input.len();
        self.deferred_len += 1;
    }

    fn store_and_delay_dry(&mut self, left: f32, right: f32) -> DryFrame {
        let len = self.dry_delay.len();
        if len == 0 {
            self.worker.mark_faulted();
            return DryFrame { left, right };
        }

        let Some(slot) = self.dry_delay.get_mut(self.dry_cursor) else {
            self.worker.mark_faulted();
            return DryFrame { left, right };
        };
        *slot = DryFrame { left, right };
        let read_cursor = (self.dry_cursor + len - self.reported_latency) % len;
        let Some(delayed) = self.dry_delay.get(read_cursor).copied() else {
            self.worker.mark_faulted();
            return DryFrame { left, right };
        };
        self.dry_cursor = (self.dry_cursor + 1) % len;
        delayed
    }

    fn poll_wet(&mut self, output_time: u64) -> Option<f32> {
        let (expected_start, expected_offset) = self.expected_output(output_time)?;

        loop {
            match self.pending_relation(expected_start) {
                PendingRelation::Matching => {
                    let wet = self.consume_pending(expected_offset);
                    if wet.is_some() {
                        self.offline_wait = None;
                        self.offline_timed_out = None;
                    }
                    return wet;
                }
                PendingRelation::Stale => self.pending_output = None,
                PendingRelation::Future | PendingRelation::Absent => break,
                PendingRelation::Invalid => {
                    self.pending_output = None;
                    self.worker.mark_faulted();
                    return None;
                }
            }
        }

        for _ in 0..self.queue_scan_limit {
            let chunk = match self.worker.pop_output() {
                Ok(chunk) => chunk,
                Err(PopError::Empty) => break,
            };
            self.pending_output = Some(chunk);

            match self.pending_relation(expected_start) {
                PendingRelation::Matching => {
                    let wet = self.consume_pending(expected_offset);
                    if wet.is_some() {
                        self.offline_wait = None;
                        self.offline_timed_out = None;
                    }
                    return wet;
                }
                PendingRelation::Stale => self.pending_output = None,
                PendingRelation::Future => return None,
                PendingRelation::Invalid => {
                    self.pending_output = None;
                    self.worker.mark_faulted();
                    return None;
                }
                PendingRelation::Absent => return None,
            }
        }

        None
    }

    fn wait_for_wet(&mut self, output_time: u64) -> Option<f32> {
        let (expected_start, _) = self.expected_output(output_time)?;
        if self.worker.is_faulted()
            || self.worker.is_discontinuous()
            || self.worker.is_stopped()
            || self.worker.requested_generation() != self.generation
        {
            self.offline_wait = None;
            return None;
        }

        if let Some(wet) = self.poll_wet(output_time) {
            return Some(wet);
        }
        if self.offline_timed_out == Some((self.generation, expected_start)) {
            return None;
        }

        let deadline = match self.offline_wait {
            Some(wait)
                if wait.generation == self.generation && wait.chunk_start == expected_start =>
            {
                wait.deadline
            }
            _ => {
                let deadline = Instant::now() + OFFLINE_OUTPUT_TIMEOUT;
                self.offline_wait = Some(OfflineWait {
                    generation: self.generation,
                    chunk_start: expected_start,
                    deadline,
                });
                deadline
            }
        };

        loop {
            if self.worker.is_faulted()
                || self.worker.is_discontinuous()
                || self.worker.is_stopped()
                || self.worker.requested_generation() != self.generation
            {
                self.offline_wait = None;
                return None;
            }
            if let Some(wet) = self.poll_wet(output_time) {
                return Some(wet);
            }
            if Instant::now() >= deadline {
                self.worker.mark_faulted();
                self.offline_wait = None;
                self.offline_timed_out = Some((self.generation, expected_start));
                return None;
            }
            self.worker.offline_wait(OFFLINE_POLL_INTERVAL);
        }
    }

    fn expected_output(&self, output_time: u64) -> Option<(u64, usize)> {
        let collection_runway = u64::try_from(self.collection_runway).ok()?;
        if output_time < collection_runway {
            return None;
        }
        let source_sample = output_time - collection_runway;
        let quantum = self.host_quantum as u64;
        let offset = (source_sample % quantum) as usize;
        Some((source_sample - offset as u64, offset))
    }

    fn pending_relation(&self, expected_start: u64) -> PendingRelation {
        let Some(chunk) = self.pending_output.as_ref() else {
            return PendingRelation::Absent;
        };
        if chunk.len() != self.host_quantum
            || chunk.start_sample() % self.host_quantum as u64 != 0
        {
            return PendingRelation::Invalid;
        }
        if chunk.generation() < self.generation
            || (chunk.generation() == self.generation && chunk.start_sample() < expected_start)
        {
            return PendingRelation::Stale;
        }
        if chunk.generation() > self.generation || chunk.start_sample() > expected_start {
            return PendingRelation::Future;
        }
        PendingRelation::Matching
    }

    fn consume_pending(&mut self, expected_offset: usize) -> Option<f32> {
        let chunk = self.pending_output.as_ref()?;
        let sample = chunk.samples().get(expected_offset).copied();
        if sample.is_none() {
            self.pending_output = None;
            self.worker.mark_faulted();
            return None;
        }
        if expected_offset.saturating_add(1) >= chunk.len() {
            self.pending_output = None;
        }
        sample
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PendingRelation {
    Absent,
    Stale,
    Matching,
    Future,
    Invalid,
}

fn sanitize_mix(mix: f32) -> f32 {
    if mix.is_finite() {
        mix.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::VecDeque;

    use super::*;
    use crate::dsp::{DspInfo, LatencyBreakdown};
    use crate::model::ModelInfo;
    #[cfg(feature = "model-ll")]
    use crate::resampler::RatePlan;

    const TEST_QUANTUM: usize = 8;
    const TEST_CORE_DELAY: usize = 3;
    const TEST_MAX_BLOCK: u32 = 1_024;
    const TEST_GENERATION: u64 = 1;

    struct FakeWorker {
        info: DspInfo,
        outputs: VecDeque<AudioChunk>,
        staged_outputs: VecDeque<AudioChunk>,
        delay: VecDeque<f32>,
        requested_generation: Cell<u64>,
        acknowledged_generation: Cell<u64>,
        attenuation: Cell<f32>,
        ready: Cell<bool>,
        faulted: Cell<bool>,
        model_faulted: Cell<bool>,
        discontinuous: Cell<bool>,
        stopped: Cell<bool>,
        always_full: bool,
        auto_output: bool,
        stage_until_offline_wait: bool,
        output_gain: f32,
        offline_wait_count: usize,
    }

    impl FakeWorker {
        fn delayed_identity(host_quantum: usize, core_delay: usize) -> Self {
            let total_host = core_delay + host_quantum * RUNWAY_QUANTA;
            Self {
                info: DspInfo {
                    model: ModelInfo {
                        sample_rate: 48_000,
                        channels: 1,
                        hop_size: host_quantum,
                        fft_size: host_quantum + core_delay,
                        lookahead: 0,
                        algorithmic_delay: core_delay,
                    },
                    host_sample_rate: 48_000,
                    host_quantum,
                    latency: LatencyBreakdown {
                        host_to_model_output_delay_model: 0,
                        model_algorithmic_delay_model: core_delay,
                        model_delay_scaled_host: core_delay,
                        model_to_host_output_delay_host: 0,
                        core_delay_host: core_delay,
                        runway_delay_host: host_quantum * RUNWAY_QUANTA,
                        total_host: total_host as u32,
                    },
                },
                outputs: VecDeque::new(),
                staged_outputs: VecDeque::new(),
                delay: std::iter::repeat(0.0).take(core_delay).collect(),
                requested_generation: Cell::new(TEST_GENERATION),
                acknowledged_generation: Cell::new(TEST_GENERATION),
                attenuation: Cell::new(100.0),
                ready: Cell::new(true),
                faulted: Cell::new(false),
                model_faulted: Cell::new(false),
                discontinuous: Cell::new(false),
                stopped: Cell::new(false),
                always_full: false,
                auto_output: true,
                stage_until_offline_wait: false,
                output_gain: 1.0,
                offline_wait_count: 0,
            }
        }

        fn reset_dsp_state(&mut self) {
            self.delay.clear();
            self.delay
                .extend(std::iter::repeat(0.0).take(self.info.latency.core_delay_host));
            self.acknowledged_generation
                .set(self.requested_generation.get());
            self.discontinuous.set(false);
        }

        fn make_output(&mut self, chunk: &AudioChunk) -> AudioChunk {
            let mut samples = [0.0; MAX_HOST_QUANTUM];
            for (index, sample) in chunk.samples().iter().copied().enumerate() {
                let delayed = self.delay.pop_front().unwrap_or(sample);
                self.delay.push_back(sample);
                samples[index] = delayed * self.output_gain;
            }
            AudioChunk::from_slice(chunk.generation(), chunk.start_sample(), &samples[..chunk.len()])
                .expect("fake output must fit the fixed chunk")
        }

        fn push_output(&mut self, chunk: AudioChunk) {
            self.outputs.push_back(chunk);
        }
    }

    impl BridgeWorker for FakeWorker {
        fn dsp_info(&self) -> DspInfo {
            self.info
        }

        fn submit(&mut self, chunk: AudioChunk) -> Result<(), SubmitError> {
            if self.faulted.get() || self.discontinuous.get() || self.stopped.get() {
                return Err(SubmitError::Unavailable(chunk));
            }
            if self.always_full {
                return Err(SubmitError::Full(chunk));
            }
            if self.requested_generation.get() != self.acknowledged_generation.get() {
                self.reset_dsp_state();
            }
            if self.auto_output {
                let output = self.make_output(&chunk);
                if self.stage_until_offline_wait {
                    self.staged_outputs.push_back(output);
                } else {
                    self.outputs.push_back(output);
                }
            }
            Ok(())
        }

        fn pop_output(&mut self) -> Result<AudioChunk, PopError> {
            self.outputs.pop_front().ok_or(PopError::Empty)
        }

        fn request_reset(&self, generation: u64) {
            self.requested_generation.set(generation);
        }

        fn set_attenuation(&self, attenuation: f32) {
            self.attenuation.set(attenuation);
        }

        fn requested_generation(&self) -> u64 {
            self.requested_generation.get()
        }

        fn acknowledged_generation(&self) -> u64 {
            self.acknowledged_generation.get()
        }

        fn is_ready(&self) -> bool {
            self.ready.get()
        }

        fn is_faulted(&self) -> bool {
            self.faulted.get()
        }

        fn is_model_faulted(&self) -> bool {
            self.model_faulted.get()
        }

        fn is_discontinuous(&self) -> bool {
            self.discontinuous.get()
        }

        fn is_stopped(&self) -> bool {
            self.stopped.get()
        }

        fn mark_faulted(&self) {
            self.faulted.set(true);
        }

        fn mark_discontinuous(&self) {
            self.discontinuous.set(true);
        }

        fn offline_wait(&mut self, _interval: Duration) {
            self.offline_wait_count += 1;
            if let Some(output) = self.staged_outputs.pop_front() {
                self.outputs.push_back(output);
            }
        }

        fn shutdown(&mut self) {
            self.ready.set(false);
            self.stopped.set(true);
        }
    }

    fn bridge_config(
        channels: usize,
        max_buffer_size: u32,
        process_mode: ProcessMode,
        worker: &FakeWorker,
    ) -> BridgeConfig {
        BridgeConfig {
            channels,
            max_buffer_size,
            host_quantum: worker.info.host_quantum,
            reported_latency: worker.info.latency.total_host,
            process_mode,
            queue_capacity: HostBridge::queue_capacity(
                max_buffer_size,
                worker.info.host_quantum,
            )
            .expect("test queue geometry must be valid"),
        }
    }

    fn make_bridge(
        channels: usize,
        process_mode: ProcessMode,
    ) -> HostBridge<FakeWorker> {
        let worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        let config = bridge_config(channels, TEST_MAX_BLOCK, process_mode, &worker);
        HostBridge::build(worker, config).expect("deterministic bridge must construct")
    }

    fn test_signal(length: usize) -> Vec<f32> {
        (0..length)
            .map(|index| ((index * 17 % 101) as f32 - 50.0) / 25.0)
            .collect()
    }

    fn delayed(source: &[f32], delay: usize) -> Vec<f32> {
        (0..source.len())
            .map(|index| index.checked_sub(delay).map_or(0.0, |source_index| source[source_index]))
            .collect()
    }

    fn render_mono(
        mut bridge: HostBridge<FakeWorker>,
        source: &[f32],
        partition: usize,
        mix: f32,
    ) -> (Vec<f32>, HostBridge<FakeWorker>) {
        let mut output = source.to_vec();
        for block in output.chunks_mut(partition) {
            bridge
                .process_mono(block, || mix)
                .expect("test mono layout must remain valid");
        }
        (output, bridge)
    }

    fn assert_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 1.0e-6,
                "sample {index}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn arbitrary_partitions_produce_identical_complete_output() {
        let source = test_signal(4_129);
        let partitions = [1, 7, 64, 127, 480, 511, 1_024];
        let (reference, _) = render_mono(
            make_bridge(1, ProcessMode::Realtime),
            &source,
            partitions[0],
            1.0,
        );
        assert_eq!(reference.len(), source.len());
        assert_close(
            &reference,
            &delayed(&source, TEST_CORE_DELAY + TEST_QUANTUM * RUNWAY_QUANTA),
        );

        for partition in partitions.into_iter().skip(1) {
            let (candidate, _) = render_mono(
                make_bridge(1, ProcessMode::Realtime),
                &source,
                partition,
                1.0,
            );
            assert_eq!(candidate.len(), reference.len());
            assert_eq!(candidate, reference, "partition {partition} changed output");
        }
    }

    #[test]
    fn mono_and_stereo_mapping_mix_per_sample_at_zero_half_and_full() {
        let source = test_signal(96);
        let latency = TEST_CORE_DELAY + TEST_QUANTUM * RUNWAY_QUANTA;
        for mix in [0.0, 0.5, 1.0] {
            let mut worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
            worker.output_gain = 2.0;
            let config = bridge_config(1, TEST_MAX_BLOCK, ProcessMode::Realtime, &worker);
            let bridge = HostBridge::build(worker, config).expect("mono bridge must construct");
            let (actual, _) = render_mono(bridge, &source, 7, mix);
            let dry = delayed(&source, latency);
            let expected: Vec<_> = dry
                .iter()
                .map(|sample| sample * (1.0 - mix) + sample * 2.0 * mix)
                .collect();
            assert_close(&actual, &expected);

            let left = test_signal(96);
            let right: Vec<_> = left.iter().map(|sample| sample * -0.25 + 0.75).collect();
            let inference: Vec<_> = left
                .iter()
                .zip(&right)
                .map(|(left, right)| (left + right) * 0.5)
                .collect();
            let dry_left = delayed(&left, latency);
            let dry_right = delayed(&right, latency);
            let wet = delayed(&inference, latency);
            let mut actual_left = left;
            let mut actual_right = right;
            let mut worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
            worker.output_gain = 2.0;
            let config = bridge_config(2, TEST_MAX_BLOCK, ProcessMode::Realtime, &worker);
            let mut bridge =
                HostBridge::build(worker, config).expect("stereo bridge must construct");
            let mut mix_calls = 0;
            for (left, right) in actual_left.chunks_mut(7).zip(actual_right.chunks_mut(7)) {
                bridge
                    .process_stereo(left, right, || {
                        mix_calls += 1;
                        mix
                    })
                    .expect("test stereo layout must remain valid");
            }
            assert_eq!(mix_calls, actual_left.len());
            let expected_left: Vec<_> = dry_left
                .iter()
                .zip(&wet)
                .map(|(dry, wet)| dry * (1.0 - mix) + wet * 2.0 * mix)
                .collect();
            let expected_right: Vec<_> = dry_right
                .iter()
                .zip(&wet)
                .map(|(dry, wet)| dry * (1.0 - mix) + wet * 2.0 * mix)
                .collect();
            assert_close(&actual_left, &expected_left);
            assert_close(&actual_right, &expected_right);
        }
    }

    #[test]
    fn late_faulted_and_overflowed_workers_return_timestamp_aligned_dry() {
        let source = test_signal(192);
        let latency = TEST_CORE_DELAY + TEST_QUANTUM * RUNWAY_QUANTA;
        let expected = delayed(&source, latency);

        let mut late_worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        late_worker.auto_output = false;
        let config = bridge_config(1, 32, ProcessMode::Realtime, &late_worker);
        let late_bridge = HostBridge::build(late_worker, config).expect("late bridge must construct");
        let (late, late_bridge) = render_mono(late_bridge, &source, 32, 1.0);
        assert_close(&late, &expected);
        assert!(!late_bridge.status().faulted);
        assert_eq!(late_bridge.worker.offline_wait_count, 0);

        let fault_worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        let config = bridge_config(1, 32, ProcessMode::Realtime, &fault_worker);
        let mut fault_bridge =
            HostBridge::build(fault_worker, config).expect("fault bridge must first construct");
        fault_bridge.worker.faulted.set(true);
        let (fault, fault_bridge) = render_mono(fault_bridge, &source, 32, 1.0);
        assert_close(&fault, &expected);
        assert!(fault_bridge.status().faulted);

        let offline_fault_worker =
            FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        let config = bridge_config(1, 32, ProcessMode::Offline, &offline_fault_worker);
        let mut offline_fault_bridge = HostBridge::build(offline_fault_worker, config)
            .expect("offline fault bridge must first construct");
        offline_fault_bridge.worker.faulted.set(true);
        let (offline_fault, offline_fault_bridge) =
            render_mono(offline_fault_bridge, &source, 32, 1.0);
        assert_close(&offline_fault, &expected);
        assert_eq!(offline_fault_bridge.worker.offline_wait_count, 0);

        let mut full_worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        full_worker.always_full = true;
        let config = bridge_config(1, 32, ProcessMode::Realtime, &full_worker);
        let full_bridge = HostBridge::build(full_worker, config).expect("full bridge must construct");
        let (full, full_bridge) = render_mono(full_bridge, &source, 32, 1.0);
        assert_close(&full, &expected);
        assert!(full_bridge.status().input_discontinuous);
    }

    #[test]
    fn stale_and_future_results_never_replace_the_aligned_dry_sample() {
        let source = test_signal(64);
        let latency = TEST_CORE_DELAY + TEST_QUANTUM * RUNWAY_QUANTA;
        let expected = delayed(&source, latency);
        let mut worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        worker.auto_output = false;
        worker.push_output(
            AudioChunk::from_slice(0, 0, &[99.0; TEST_QUANTUM])
                .expect("stale test chunk must fit"),
        );
        worker.push_output(
            AudioChunk::from_slice(
                TEST_GENERATION,
                TEST_QUANTUM as u64,
                &[77.0; TEST_QUANTUM],
            )
            .expect("future test chunk must fit"),
        );
        let config = bridge_config(1, TEST_MAX_BLOCK, ProcessMode::Realtime, &worker);
        let bridge = HostBridge::build(worker, config).expect("timeline bridge must construct");
        let (actual, _) = render_mono(bridge, &source, 7, 1.0);

        assert_eq!(actual[latency], expected[latency]);
        assert_ne!(actual[latency], 99.0);
        assert_ne!(actual[latency], 77.0);
    }

    #[test]
    fn reset_generation_discards_old_output_and_matches_a_fresh_bridge() {
        let source = test_signal(257);
        let mut bridge = make_bridge(1, ProcessMode::Realtime);
        let prefix = test_signal(73);
        let mut prefix_in_place = prefix;
        for block in prefix_in_place.chunks_mut(7) {
            bridge
                .process_mono(block, || 1.0)
                .expect("prefix must process");
        }
        let old_generation = bridge.generation();
        bridge.worker.push_output(
            AudioChunk::from_slice(old_generation, 0, &[123.0; TEST_QUANTUM])
                .expect("old output must fit"),
        );
        bridge.reset();
        assert_eq!(bridge.generation(), old_generation + 1);
        assert_eq!(bridge.worker.requested_generation(), bridge.generation());

        let (after_reset, bridge) = render_mono(bridge, &source, 127, 1.0);
        let (fresh, _) = render_mono(make_bridge(1, ProcessMode::Realtime), &source, 127, 1.0);
        assert_eq!(bridge.status().acknowledged_generation, bridge.generation());
        assert_eq!(after_reset, fresh);
        assert!(!after_reset.contains(&123.0));
    }

    #[test]
    fn realtime_buffered_and_offline_share_output_but_only_offline_waits() {
        let source = test_signal(192);
        let (realtime, realtime_bridge) = render_mono(
            make_bridge(1, ProcessMode::Realtime),
            &source,
            64,
            1.0,
        );
        let (buffered, buffered_bridge) = render_mono(
            make_bridge(1, ProcessMode::Buffered),
            &source,
            64,
            1.0,
        );
        assert_eq!(realtime_bridge.worker.offline_wait_count, 0);
        assert_eq!(buffered_bridge.worker.offline_wait_count, 0);

        let mut offline_worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        offline_worker.stage_until_offline_wait = true;
        let config = bridge_config(1, TEST_MAX_BLOCK, ProcessMode::Offline, &offline_worker);
        let offline_bridge =
            HostBridge::build(offline_worker, config).expect("offline bridge must construct");
        let (offline, offline_bridge) = render_mono(offline_bridge, &source, 64, 1.0);
        assert!(offline_bridge.worker.offline_wait_count > 0);
        assert_eq!(buffered, realtime);
        assert_eq!(offline, realtime);
    }

    #[test]
    fn invalid_construction_and_process_shapes_fail_without_touching_audio() {
        assert_eq!(
            HostBridge::queue_capacity(0, TEST_QUANTUM),
            Err(BridgeConfigError::ZeroMaximumBufferSize)
        );
        assert_eq!(
            HostBridge::queue_capacity(32, 0),
            Err(BridgeConfigError::InvalidHostQuantum)
        );

        let worker = FakeWorker::delayed_identity(TEST_QUANTUM, TEST_CORE_DELAY);
        worker.ready.set(false);
        let config = bridge_config(1, 32, ProcessMode::Realtime, &worker);
        assert!(matches!(
            HostBridge::build(worker, config),
            Err(BridgeConfigError::WorkerUnavailable)
        ));

        let mut bridge = make_bridge(2, ProcessMode::Realtime);
        let mut unchanged = vec![1.0, -2.0, 3.0];
        let original = unchanged.clone();
        assert_eq!(
            bridge.process_mono(&mut unchanged, || 1.0),
            Err(BridgeProcessError::ChannelMismatch)
        );
        assert_eq!(unchanged, original);

        let mut bridge = make_bridge(1, ProcessMode::Realtime);
        let mut oversized = vec![0.25; TEST_MAX_BLOCK as usize + 1];
        let original = oversized.clone();
        assert_eq!(
            bridge.process_mono(&mut oversized, || 1.0),
            Err(BridgeProcessError::BlockTooLarge)
        );
        assert_eq!(oversized, original);
    }

    #[cfg(feature = "model-ll")]
    fn make_real_offline_bridge(host_sample_rate: usize) -> (HostBridge, DspInfo) {
        let plan = RatePlan::preflight(host_sample_rate)
            .expect("declared real-model rate must preflight");
        let queue_capacity = HostBridge::queue_capacity(TEST_MAX_BLOCK, plan.host_quantum)
            .expect("real-model queue geometry must be bounded");
        let worker = WorkerHandle::start(
            queue_capacity,
            host_sample_rate,
            TEST_GENERATION,
            0.0,
        )
        .expect("real worker must finish its bounded startup handshake");
        let info = worker.dsp_info();
        let config = BridgeConfig {
            channels: 1,
            max_buffer_size: TEST_MAX_BLOCK,
            host_quantum: info.host_quantum,
            reported_latency: info.latency.total_host,
            process_mode: ProcessMode::Offline,
            queue_capacity,
        };
        let bridge = HostBridge::new(worker, config)
            .expect("real Offline bridge must finish checked construction");
        (bridge, info)
    }

    #[cfg(feature = "model-ll")]
    fn render_real_offline(bridge: &mut HostBridge, source: &[f32], mix: f32) -> Vec<f32> {
        let mut rendered = source.to_vec();
        for block in rendered.chunks_mut(127) {
            bridge
                .process_mono(block, || mix)
                .expect("bounded real Offline block must process");
        }
        rendered
    }

    #[cfg(feature = "model-ll")]
    fn absolute_peak(samples: &[f32]) -> (usize, f32) {
        samples
            .iter()
            .copied()
            .enumerate()
            .map(|(index, sample)| (index, sample.abs()))
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .expect("peak fixture must not be empty")
    }

    #[cfg(feature = "model-ll")]
    #[test]
    fn real_offline_impulses_match_live_reported_latency_and_mix_alignment() {
        let _serial = crate::test_support::serialize_real_model();
        let cases = [
            (48_000, 480, 0, 0, 480, 480, 960, 1_440, 0_usize),
            (44_100, 441, 240, 220, 662, 882, 882, 1_764, 1_usize),
            (96_000, 960, 240, 480, 1_440, 1_920, 1_920, 3_840, 1_usize),
        ];

        for (
            host_rate,
            host_quantum,
            host_to_model_delay,
            model_to_host_delay,
            scaled_model_delay,
            core_delay,
            runway_delay,
            total_latency,
            tolerance,
        ) in cases
        {
            let (mut bridge, info) = make_real_offline_bridge(host_rate);
            assert_eq!(info.host_sample_rate, host_rate);
            assert_eq!(info.host_quantum, host_quantum);
            assert_eq!(info.model.sample_rate, 48_000);
            assert_eq!(info.model.channels, 1);
            assert_eq!(info.model.hop_size, 480);
            assert_eq!(info.model.fft_size, 960);
            assert_eq!(info.model.lookahead, 0);
            assert_eq!(info.model.algorithmic_delay, 480);
            assert_eq!(
                info.latency.host_to_model_output_delay_model,
                host_to_model_delay
            );
            assert_eq!(
                info.latency.model_to_host_output_delay_host,
                model_to_host_delay
            );
            assert_eq!(info.latency.model_delay_scaled_host, scaled_model_delay);
            assert_eq!(info.latency.core_delay_host, core_delay);
            assert_eq!(info.latency.runway_delay_host, runway_delay);
            assert_eq!(info.latency.total_host, total_latency);
            assert_eq!(bridge.reported_latency(), info.latency.total_host);

            let fixture_len = total_latency as usize + host_quantum * 3;
            let mut impulse = vec![0.0; fixture_len];
            impulse[0] = 1.0;
            let mut peaks = Vec::new();
            for (run, mix) in [0.0, 0.5, 1.0].into_iter().enumerate() {
                if run != 0 {
                    bridge.reset();
                }
                bridge.set_attenuation(0.0);
                let rendered = render_real_offline(&mut bridge, &impulse, mix);
                assert_eq!(rendered.len(), impulse.len());
                assert!(rendered.iter().all(|sample| sample.is_finite()));
                let peak = absolute_peak(&rendered);
                assert!(peak.1 > 1.0e-4);
                assert!(peak.0.abs_diff(total_latency as usize) <= tolerance);
                peaks.push(peak.0);
                let status = bridge.status();
                assert!(status.ready);
                assert!(!status.faulted);
                assert!(!status.model_degraded);
                assert!(!status.input_discontinuous);
                assert_eq!(status.acknowledged_generation, bridge.generation());
            }
            assert_eq!(peaks[0], total_latency as usize);
            assert!(peaks[0].abs_diff(peaks[1]) <= tolerance);
            assert!(peaks[0].abs_diff(peaks[2]) <= tolerance);
            bridge.shutdown();
        }
    }

    #[cfg(feature = "model-ll")]
    #[test]
    fn two_real_offline_runs_are_identical_after_generation_reset() {
        let _serial = crate::test_support::serialize_real_model();
        let (mut bridge, info) = make_real_offline_bridge(48_000);
        let length = info.latency.total_host as usize + info.host_quantum * 5;
        let fixture: Vec<_> = (0..length)
            .map(|index| {
                if index % 317 == 0 {
                    0.5
                } else {
                    let phase = index as f32 / info.host_sample_rate as f32;
                    0.1 * (phase * 733.0 * std::f32::consts::TAU).sin()
                }
            })
            .collect();

        bridge.set_attenuation(0.0);
        let first = render_real_offline(&mut bridge, &fixture, 1.0);
        bridge.reset();
        bridge.set_attenuation(0.0);
        let second = render_real_offline(&mut bridge, &fixture, 1.0);
        assert_eq!(second, first);
        assert!(first.iter().any(|sample| sample.abs() > 1.0e-8));
        let status = bridge.status();
        assert!(!status.faulted);
        assert!(!status.model_degraded);
        assert!(!status.input_discontinuous);
        assert_eq!(status.acknowledged_generation, bridge.generation());
        bridge.shutdown();
    }
}
