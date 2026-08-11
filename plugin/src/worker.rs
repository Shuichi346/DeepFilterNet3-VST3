//! Lock-free host-to-worker transport for DeepFilterNet inference.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rtrb::{Consumer, PopError, Producer, PushError, RingBuffer};

use crate::dsp::{DspCore, DspInfo};
use crate::model::ModelInfo;

/// Largest host chunk accepted by the fixed worker transport.
pub(crate) const MAX_HOST_QUANTUM: usize = 1_920;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const IDLE_WAIT: Duration = Duration::from_micros(100);
const MAX_QUEUE_MEMORY_BYTES: usize = 16 * 1024 * 1024;

/// A timestamped mono chunk moved by value through the SPSC queues.
#[derive(Clone, Debug)]
pub(crate) struct AudioChunk {
    generation: u64,
    start_sample: u64,
    len: usize,
    samples: [f32; MAX_HOST_QUANTUM],
}

impl AudioChunk {
    /// Copy a bounded mono slice into a fixed-size transport message.
    pub(crate) fn from_slice(
        generation: u64,
        start_sample: u64,
        samples: &[f32],
    ) -> Result<Self, AudioChunkError> {
        if samples.len() > MAX_HOST_QUANTUM {
            return Err(AudioChunkError::TooLong);
        }

        let mut message = Self {
            generation,
            start_sample,
            len: samples.len(),
            samples: [0.0; MAX_HOST_QUANTUM],
        };
        message.samples[..message.len].copy_from_slice(samples);
        Ok(message)
    }

    /// Construct a bounded silent message when a caller needs an initialized chunk.
    pub(crate) fn silence(
        generation: u64,
        start_sample: u64,
        len: usize,
    ) -> Result<Self, AudioChunkError> {
        if len > MAX_HOST_QUANTUM {
            return Err(AudioChunkError::TooLong);
        }
        Ok(Self {
            generation,
            start_sample,
            len,
            samples: [0.0; MAX_HOST_QUANTUM],
        })
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn start_sample(&self) -> u64 {
        self.start_sample
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn samples(&self) -> &[f32] {
        debug_assert!(self.is_well_formed());
        &self.samples[..self.len]
    }

    fn is_well_formed(&self) -> bool {
        self.len <= MAX_HOST_QUANTUM
    }
}

/// Construction error for an `AudioChunk` that exceeds its fixed capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AudioChunkError {
    TooLong,
}

/// Atomics shared between the callback-facing handle and its worker thread.
#[derive(Debug)]
pub(crate) struct WorkerControl {
    requested_generation: AtomicU64,
    acknowledged_generation: AtomicU64,
    attenuation_bits: AtomicU32,
    ready: AtomicBool,
    faulted: AtomicBool,
    model_faulted: AtomicBool,
    discontinuous: AtomicBool,
    stop_requested: AtomicBool,
    stopped: AtomicBool,
}

impl WorkerControl {
    fn new(initial_generation: u64, attenuation: f32) -> Self {
        Self {
            requested_generation: AtomicU64::new(initial_generation),
            acknowledged_generation: AtomicU64::new(initial_generation),
            attenuation_bits: AtomicU32::new(attenuation.to_bits()),
            ready: AtomicBool::new(false),
            faulted: AtomicBool::new(false),
            model_faulted: AtomicBool::new(false),
            discontinuous: AtomicBool::new(false),
            stop_requested: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
        }
    }

    /// Request a logical reset without touching queues or worker-owned state.
    pub(crate) fn request_reset(&self, generation: u64) {
        self.requested_generation.store(generation, Ordering::Release);
    }

    /// Publish the latest host parameter; it is independent of lifecycle state.
    pub(crate) fn set_attenuation(&self, attenuation: f32) {
        self.attenuation_bits
            .store(attenuation.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn requested_generation(&self) -> u64 {
        self.requested_generation.load(Ordering::Acquire)
    }

    pub(crate) fn acknowledged_generation(&self) -> u64 {
        self.acknowledged_generation.load(Ordering::Acquire)
    }

    pub(crate) fn attenuation(&self) -> f32 {
        f32::from_bits(self.attenuation_bits.load(Ordering::Relaxed))
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub(crate) fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
    }

    pub(crate) fn is_model_faulted(&self) -> bool {
        self.model_faulted.load(Ordering::Acquire)
    }

    pub(crate) fn is_discontinuous(&self) -> bool {
        self.discontinuous.load(Ordering::Acquire)
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
}

/// A fully handshaken worker with host-owned SPSC endpoints only.
pub(crate) struct WorkerHandle {
    input: Producer<AudioChunk>,
    output: Consumer<AudioChunk>,
    control: Arc<WorkerControl>,
    thread: Option<JoinHandle<()>>,
    info: DspInfo,
}

impl WorkerHandle {
    /// Start a worker transactionally. A returned handle always has a live model engine.
    pub(crate) fn start(
        queue_capacity: usize,
        host_sample_rate: usize,
        initial_generation: u64,
        attenuation: f32,
    ) -> Result<Self, WorkerError> {
        validate_queue_capacity(queue_capacity)?;

        let (input, worker_input) = RingBuffer::new(queue_capacity);
        let (worker_output, output) = RingBuffer::new(queue_capacity);
        let control = Arc::new(WorkerControl::new(initial_generation, attenuation));
        let worker_control = Arc::clone(&control);
        let (startup_tx, startup_rx) = mpsc::sync_channel(1);

        let thread = thread::Builder::new()
            .name("deepfilter-inference".to_owned())
            .spawn(move || {
                worker_entry(
                    worker_input,
                    worker_output,
                    worker_control,
                    startup_tx,
                    host_sample_rate,
                )
            })
            .map_err(|error| WorkerError::new(format!("could not spawn DeepFilterNet worker: {error}")))?;

        match startup_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(info)) => Ok(Self {
                input,
                output,
                control,
                thread: Some(thread),
                info,
            }),
            Ok(Err(error)) => {
                stop_and_dispose(thread, &control);
                Err(WorkerError::new(error))
            }
            Err(RecvTimeoutError::Timeout) => {
                stop_and_dispose(thread, &control);
                Err(WorkerError::new("DeepFilterNet worker startup timed out"))
            }
            Err(RecvTimeoutError::Disconnected) => {
                stop_and_dispose(thread, &control);
                Err(WorkerError::new("DeepFilterNet worker exited before startup"))
            }
        }
    }

    pub(crate) fn model_info(&self) -> ModelInfo {
        self.info.model
    }

    pub(crate) fn dsp_info(&self) -> DspInfo {
        self.info
    }

    pub(crate) fn host_quantum(&self) -> usize {
        self.info.host_quantum
    }

    pub(crate) fn reported_latency(&self) -> u32 {
        self.info.latency.total_host
    }

    /// Submit without waiting; a full queue returns ownership of the original message.
    pub(crate) fn submit(&mut self, chunk: AudioChunk) -> Result<(), SubmitError> {
        if self.control.is_faulted()
            || self.control.is_discontinuous()
            || self.control.is_stopped()
        {
            return Err(SubmitError::Unavailable(chunk));
        }

        match self.input.push(chunk) {
            Ok(()) => Ok(()),
            Err(PushError::Full(chunk)) => Err(SubmitError::Full(chunk)),
        }
    }

    /// Pop one completed worker result without waiting.
    pub(crate) fn pop_output(&mut self) -> Result<AudioChunk, PopError> {
        self.output.pop()
    }

    /// Request a reset through atomics only; the worker acknowledges after restoring pristine state.
    pub(crate) fn request_reset(&self, generation: u64) {
        self.control.request_reset(generation);
    }

    /// Publish attenuation through atomics only.
    pub(crate) fn set_attenuation(&self, attenuation: f32) {
        self.control.set_attenuation(attenuation);
    }

    pub(crate) fn requested_generation(&self) -> u64 {
        self.control.requested_generation()
    }

    pub(crate) fn acknowledged_generation(&self) -> u64 {
        self.control.acknowledged_generation()
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.control.is_ready()
    }

    pub(crate) fn is_faulted(&self) -> bool {
        self.control.is_faulted()
    }

    pub(crate) fn is_model_faulted(&self) -> bool {
        self.control.is_model_faulted()
    }

    pub(crate) fn is_discontinuous(&self) -> bool {
        self.control.is_discontinuous()
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.control.is_stopped()
    }

    /// Publish an offline deadline or bridge integrity fault through an atomic only.
    pub(crate) fn mark_faulted(&self) {
        self.control.faulted.store(true, Ordering::Release);
    }

    /// Stop accepting the current generation after an input queue discontinuity.
    pub(crate) fn mark_discontinuous(&self) {
        self.control.discontinuous.store(true, Ordering::Release);
    }

    /// Stop during deactivation/reinitialization/drop; never call this on the audio callback.
    pub(crate) fn shutdown(&mut self) {
        self.control.stop_requested.store(true, Ordering::Release);
        if let Some(thread) = self.thread.as_ref() {
            thread.thread().unpark();
        }
        if let Some(thread) = self.thread.take() {
            stop_and_dispose(thread, &self.control);
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Immediate submission failure that retains the original fixed-size message.
#[derive(Debug)]
pub(crate) enum SubmitError {
    Full(AudioChunk),
    Unavailable(AudioChunk),
}

impl SubmitError {
    pub(crate) fn into_chunk(self) -> AudioChunk {
        match self {
            Self::Full(chunk) | Self::Unavailable(chunk) => chunk,
        }
    }
}

/// Owned startup failure suitable for selecting the host's bypass state.
#[derive(Debug)]
pub(crate) struct WorkerError(String);

impl WorkerError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WorkerError {}

fn validate_queue_capacity(capacity: usize) -> Result<(), WorkerError> {
    if capacity == 0 {
        return Err(WorkerError::new("worker queue capacity must not be zero"));
    }
    let requested_bytes = capacity
        .checked_mul(2)
        .and_then(|slots| slots.checked_mul(std::mem::size_of::<AudioChunk>()))
        .ok_or_else(|| WorkerError::new("worker queue capacity is too large"))?;
    if requested_bytes > MAX_QUEUE_MEMORY_BYTES {
        return Err(WorkerError::new(
            "worker queue memory exceeds the bounded initialization limit",
        ));
    }
    Ok(())
}

fn worker_entry(
    mut input: Consumer<AudioChunk>,
    mut output: Producer<AudioChunk>,
    control: Arc<WorkerControl>,
    startup: mpsc::SyncSender<Result<DspInfo, String>>,
    host_sample_rate: usize,
) {
    let startup_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = crate::model::DfEngine::new().map_err(|error| error.to_string())?;
        DspCore::new(engine, host_sample_rate).map_err(|error| error.to_string())
    }));
    let mut core = match startup_result {
        Ok(Ok(core)) => core,
        Ok(Err(error)) => {
            control.faulted.store(true, Ordering::Release);
            control.stopped.store(true, Ordering::Release);
            let _ = startup.send(Err(error.to_string()));
            return;
        }
        Err(_) => {
            control.faulted.store(true, Ordering::Release);
            control.stopped.store(true, Ordering::Release);
            let _ = startup.send(Err("DeepFilterNet worker panicked during startup".to_owned()));
            return;
        }
    };

    let info = core.info();
    control.ready.store(true, Ordering::Release);
    if startup.send(Ok(info)).is_err() {
        control.stop_requested.store(true, Ordering::Release);
    }

    let loop_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        worker_loop(&mut core, &mut input, &mut output, &control)
    }));
    if loop_result.is_err() {
        control.faulted.store(true, Ordering::Release);
    }
    control.ready.store(false, Ordering::Release);
    control.stopped.store(true, Ordering::Release);
}

fn worker_loop(
    core: &mut DspCore,
    input: &mut Consumer<AudioChunk>,
    output: &mut Producer<AudioChunk>,
    control: &WorkerControl,
) {
    let mut active_generation = control.acknowledged_generation.load(Ordering::Acquire);
    let mut expected_input_start = 0_u64;

    while !control.stop_requested.load(Ordering::Acquire) {
        let requested_generation = control.requested_generation.load(Ordering::Acquire);
        if requested_generation != active_generation {
            core.reset();
            active_generation = requested_generation;
            control.model_faulted.store(false, Ordering::Release);
            control.discontinuous.store(false, Ordering::Release);
            expected_input_start = 0;
            control
                .acknowledged_generation
                .store(active_generation, Ordering::Release);
            continue;
        }

        let chunk = match input.pop() {
            Ok(chunk) => chunk,
            Err(PopError::Empty) => {
                thread::park_timeout(IDLE_WAIT);
                continue;
            }
        };

        if chunk.generation() != active_generation {
            let requested_generation = control.requested_generation.load(Ordering::Acquire);
            if chunk.generation() != requested_generation {
                continue;
            }

            core.reset();
            active_generation = requested_generation;
            control.model_faulted.store(false, Ordering::Release);
            control.discontinuous.store(false, Ordering::Release);
            expected_input_start = 0;
            control
                .acknowledged_generation
                .store(active_generation, Ordering::Release);
        }
        if !chunk.is_well_formed() || chunk.len() != core.info().host_quantum {
            control.faulted.store(true, Ordering::Release);
            break;
        }
        if chunk.start_sample() != expected_input_start {
            control.discontinuous.store(true, Ordering::Release);
            continue;
        }

        let attenuation = control.attenuation();
        let mut processed = match AudioChunk::silence(
            chunk.generation(),
            chunk.start_sample(),
            core.info().host_quantum,
        ) {
            Ok(chunk) => chunk,
            Err(_) => {
                control.faulted.store(true, Ordering::Release);
                break;
            }
        };
        let output_len = core.info().host_quantum;
        let outcome = match core.process_chunk(
            chunk.samples(),
            attenuation,
            &mut processed.samples[..output_len],
        ) {
            Ok(outcome) => outcome,
            Err(_) => {
                control.faulted.store(true, Ordering::Release);
                break;
            }
        };
        if outcome.model_faulted {
            control.model_faulted.store(true, Ordering::Release);
        }
        expected_input_start = match expected_input_start
            .checked_add(core.info().host_quantum as u64)
        {
            Some(next) => next,
            None => {
                control.faulted.store(true, Ordering::Release);
                break;
            }
        };
        // Full output is an intentional nonblocking drop. A later bridge uses dry fallback.
        let _ = output.push(processed);
    }
}

fn stop_and_dispose(thread: JoinHandle<()>, control: &WorkerControl) {
    control.stop_requested.store(true, Ordering::Release);
    thread.thread().unpark();

    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    let mut thread = Some(thread);
    while let Some(handle) = thread.as_ref() {
        if handle.is_finished() {
            let handle = thread.take().expect("join handle disappeared");
            if handle.join().is_err() {
                control.faulted.store(true, Ordering::Release);
            }
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_chunks_preserve_timeline_and_reject_oversized_payloads() {
        let samples = [1.0, -2.0, 3.5];
        let chunk = AudioChunk::from_slice(7, 480, &samples).expect("small chunk must fit");
        assert_eq!(chunk.generation(), 7);
        assert_eq!(chunk.start_sample(), 480);
        assert_eq!(chunk.len(), samples.len());
        assert_eq!(chunk.samples(), samples);
        assert!(matches!(
            AudioChunk::from_slice(0, 0, &[0.0; MAX_HOST_QUANTUM + 1]),
            Err(AudioChunkError::TooLong)
        ));
        assert!(matches!(
            AudioChunk::silence(0, 0, MAX_HOST_QUANTUM + 1),
            Err(AudioChunkError::TooLong)
        ));
    }

    #[test]
    fn reset_and_parameter_publication_are_explicit_finite_handshakes() {
        let control = WorkerControl::new(4, 20.0);
        assert_eq!(control.requested_generation(), 4);
        assert_eq!(control.acknowledged_generation(), 4);
        assert!(!control.is_ready());
        assert!(!control.is_faulted());
        assert!(!control.is_model_faulted());
        assert!(!control.is_discontinuous());
        assert!(!control.is_stopped());

        control.request_reset(5);
        assert_eq!(control.requested_generation(), 5);
        assert_eq!(control.acknowledged_generation(), 4);
        control
            .acknowledged_generation
            .store(control.requested_generation(), Ordering::Release);
        assert_eq!(control.acknowledged_generation(), 5);

        control.set_attenuation(37.5);
        assert_eq!(control.attenuation(), 37.5);
    }

    #[test]
    fn queue_memory_validation_is_checked_and_bounded() {
        assert!(validate_queue_capacity(0).is_err());
        let bytes_per_capacity = std::mem::size_of::<AudioChunk>() * 2;
        let largest_valid = MAX_QUEUE_MEMORY_BYTES / bytes_per_capacity;
        assert!(largest_valid > 0);
        assert!(validate_queue_capacity(largest_valid).is_ok());
        assert!(validate_queue_capacity(largest_valid + 1).is_err());
        assert!(validate_queue_capacity(usize::MAX).is_err());
    }
}
