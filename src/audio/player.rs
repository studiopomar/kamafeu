#[cfg(not(target_arch = "wasm32"))]
use crate::audio::playback_timeline::{PlaybackTimeline, TimelineChunk, TimelineDecision};
use std::sync::atomic::{AtomicBool, Ordering};

static FLUSH_DENORMALS_TO_ZERO: AtomicBool = AtomicBool::new(true);

pub fn set_flush_denormals_to_zero(enabled: bool) {
    FLUSH_DENORMALS_TO_ZERO.store(enabled, Ordering::Relaxed);
}

#[cfg(not(target_arch = "wasm32"))]
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackSchedulerEvent {
    PlaybackStarted {
        generation: u64,
        start_ms: f64,
    },
    ChunkAccepted {
        generation: u64,
        index: usize,
        start_ms: f64,
        end_ms: f64,
    },
    ChunkRejected {
        generation: u64,
        index: usize,
        reason: String,
    },
    WaitingForChunk {
        generation: u64,
        played_ms: f64,
    },
    PlaybackResumed {
        generation: u64,
        played_ms: f64,
    },
    Underrun {
        generation: u64,
        played_ms: f64,
    },
    PlaybackFinished {
        generation: u64,
        played_ms: f64,
    },
    PlaybackStopped {
        generation: u64,
        played_ms: f64,
    },
    ProducerClosed {
        generation: u64,
        final_chunk_received: bool,
        played_ms: f64,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    active_sink: Option<Sink>,
    volume: f32,
    speed: f32,
    scheduler_tx: Option<Sender<SchedulerCommand>>,
    scheduler_state: Arc<std::sync::atomic::AtomicU8>,
    scheduler_position_ms: Arc<std::sync::atomic::AtomicU64>,
    scheduler_queued_until_ms: Arc<std::sync::atomic::AtomicU64>,
    playback_generation: Arc<std::sync::atomic::AtomicU64>,
    scheduler_events: Arc<Mutex<VecDeque<PlaybackSchedulerEvent>>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct ProgressivePlaybackHandle {
    tx: Sender<SchedulerCommand>,
    volume: f32,
    speed: f32,
    generation: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProgressivePlaybackHandle {
    /// Informa ao scheduler que a thread produtora terminou. Um chunk final
    /// aceito encerra normalmente; sem ele, a geração é incompleta e não
    /// pode permanecer indefinidamente em `Waiting`.
    pub fn producer_closed(&self, final_chunk_received: bool) {
        if self
            .tx
            .send(SchedulerCommand::ProducerClosed {
                generation: self.generation,
                final_chunk_received,
            })
            .is_err()
        {
            crate::terminal_log::error("Audio", "scheduler bridge could not close producer");
        }
    }

    pub fn start(
        &self,
        index: usize,
        start_ms: f64,
        end_ms: f64,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
    ) {
        if self
            .tx
            .send(SchedulerCommand::Start {
                generation: self.generation,
                index,
                start_ms,
                end_ms,
                samples,
                sample_rate,
                channels,
                final_chunk,
                volume: self.volume,
                speed: self.speed,
            })
            .is_err()
        {
            crate::terminal_log::error("Audio", "scheduler bridge could not send initial chunk");
        }
    }

    pub fn append(
        &self,
        index: usize,
        start_ms: f64,
        end_ms: f64,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
    ) {
        if self
            .tx
            .send(SchedulerCommand::Append {
                generation: self.generation,
                index,
                start_ms,
                end_ms,
                samples,
                sample_rate,
                channels,
                final_chunk,
            })
            .is_err()
        {
            crate::terminal_log::error(
                "Audio",
                format!("scheduler bridge could not send chunk #{index}"),
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlaybackSchedulerState {
    Idle = 0,
    Running = 1,
    Waiting = 2,
    Finished = 3,
    Stopped = 4,
    Underrun = 5,
}

#[cfg(not(target_arch = "wasm32"))]
impl PlaybackSchedulerState {
    fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Running,
            2 => Self::Waiting,
            3 => Self::Finished,
            4 => Self::Stopped,
            5 => Self::Underrun,
            _ => Self::Idle,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn push_scheduler_event(
    events: &Arc<Mutex<VecDeque<PlaybackSchedulerEvent>>>,
    event: PlaybackSchedulerEvent,
) {
    if let Ok(mut queue) = events.lock() {
        queue.push_back(event);
        while queue.len() > 512 {
            queue.pop_front();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
enum SchedulerCommand {
    Start {
        generation: u64,
        index: usize,
        start_ms: f64,
        end_ms: f64,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
        volume: f32,
        speed: f32,
    },
    Append {
        generation: u64,
        index: usize,
        start_ms: f64,
        end_ms: f64,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
    },
    SetSpeed(f32),
    Stop {
        generation: u64,
    },
    ProducerClosed {
        generation: u64,
        final_chunk_received: bool,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl AudioPlayer {
    pub fn progressive_handle(&self) -> Option<ProgressivePlaybackHandle> {
        self.scheduler_tx
            .as_ref()
            .map(|tx| ProgressivePlaybackHandle {
                tx: tx.clone(),
                volume: self.volume,
                speed: self.speed,
                generation: self.playback_generation.load(Ordering::Acquire),
            })
    }

    pub fn begin_progressive_generation(&mut self) -> u64 {
        self.playback_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Generation currently owned by the transport. Scheduler events can be
    /// delivered to the UI after a newer Play request has already started.
    pub fn current_playback_generation(&self) -> u64 {
        self.playback_generation.load(Ordering::Acquire)
    }

    pub fn set_flush_denormals_to_zero(&mut self, enabled: bool) {
        set_flush_denormals_to_zero(enabled);
    }

    pub fn new() -> Self {
        crate::terminal_log::info("Audio", "inicializando saída CoreAudio na thread principal");
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => {
                crate::terminal_log::info("Audio", "dispositivo CoreAudio inicializado");
                (Some(s), Some(h))
            }
            Err(e) => {
                crate::terminal_log::error("Audio", format!("falha ao inicializar CoreAudio: {e}"));
                (None, None)
            }
        };

        let scheduler_state = Arc::new(std::sync::atomic::AtomicU8::new(
            PlaybackSchedulerState::Idle as u8,
        ));
        let scheduler_position_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let scheduler_queued_until_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let playback_generation = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let scheduler_events = Arc::new(Mutex::new(VecDeque::new()));
        let (scheduler_tx, scheduler_rx) = mpsc::channel();
        let worker_state = scheduler_state.clone();
        let worker_position_ms = scheduler_position_ms.clone();
        let worker_queued_until_ms = scheduler_queued_until_ms.clone();
        let worker_generation = playback_generation.clone();
        let worker_events = scheduler_events.clone();
        let worker_handle = stream_handle.clone();
        std::thread::Builder::new()
            .name("kamafeu-audio-scheduler".to_string())
            .spawn(move || {
                let Some(worker_handle) = worker_handle else {
                    return;
                };
                let mut sink: Option<Sink> = None;
                let mut final_chunk = false;
                let mut expected_sample_rate: Option<u32> = None;
                let mut expected_channels: Option<u16> = None;
                let mut active_generation = 0_u64;
                let mut expected_index = 0_usize;
                let mut expected_start_ms = 0.0_f64;
                // Sink::get_pos() is relative to the source currently being
                // played. A progressive render appends a new source roughly
                // every two seconds, so using it directly makes the visual
                // playhead jump back to zero at each chunk boundary. Keep a
                // timeline clock for the whole scheduler generation instead.
                let mut timeline_position_ms = 0.0_f64;
                let mut last_clock_tick = std::time::Instant::now();
                let mut playback_speed = 1.0_f64;
                let mut timeline_end_ms = 0.0_f64;
                let mut timeline_start_ms = 0.0_f64;
                let mut timeline = PlaybackTimeline::default();
                loop {
                    let command =
                        match scheduler_rx.recv_timeout(std::time::Duration::from_millis(20)) {
                            Ok(command) => Some(command),
                            Err(mpsc::RecvTimeoutError::Timeout) => None,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        };
                    if let Some(command) = command {
                        match command {
                            SchedulerCommand::Start {
                                generation,
                                index,
                                start_ms,
                                end_ms,
                                samples,
                                sample_rate,
                                channels,
                                final_chunk: is_final,
                                volume,
                                speed,
                            } => {
                                crate::terminal_log::debug(
                                    "Audio",
                                    format!(
                                    "received generation={} chunk={} start={:.1}ms end={:.1}ms frames={} final={}",
                                    generation,
                                    index,
                                    start_ms,
                                    end_ms,
                                    samples.len() / usize::from(channels.max(1)),
                                    is_final
                                ));
                                // A new generation always supersedes the
                                // previous sink, even when its first chunk is
                                // rejected. Otherwise an invalid restart can
                                // leave old audio playing while the scheduler
                                // reports Stopped.
                                if generation != worker_generation.load(Ordering::Acquire) {
                                    continue;
                                }
                                if let Some(old) = sink.take() {
                                    old.stop();
                                }
                                if sample_rate == 0
                                    || channels == 0
                                    || !start_ms.is_finite()
                                    || !end_ms.is_finite()
                                    || end_ms <= start_ms
                                    || samples.len() % usize::from(channels.max(1)) != 0
                                    || ((samples.len() as f64
                                        / channels.max(1) as f64
                                        * 1000.0
                                        / sample_rate as f64)
                                        - (end_ms - start_ms))
                                        .abs()
                                        > 1.0
                                {
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::ChunkRejected {
                                            generation,
                                            index,
                                            reason: "formato ou timeline inválida".to_string(),
                                        },
                                    );
                                    crate::terminal_log::warn("Audio", format!("rejected initial chunk: rate={sample_rate} channels={channels}"));
                                    worker_state.store(
                                        PlaybackSchedulerState::Stopped as u8,
                                        Ordering::Release,
                                    );
                                    expected_sample_rate = None;
                                    expected_channels = None;
                                    active_generation = 0;
                                    expected_index = 0;
                                    expected_start_ms = 0.0;
                                    timeline = PlaybackTimeline::default();
                                    worker_position_ms.store(0, Ordering::Release);
                                    worker_queued_until_ms.store(0, Ordering::Release);
                                    continue;
                                }
                                if timeline.start(TimelineChunk {
                                    generation,
                                    index,
                                    start_ms,
                                    end_ms,
                                    is_final,
                                }) != TimelineDecision::Accepted
                                {
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::ChunkRejected {
                                            generation,
                                            index,
                                            reason: "timeline validator rejected start".to_string(),
                                        },
                                    );
                                    expected_sample_rate = None;
                                    expected_channels = None;
                                    active_generation = 0;
                                    expected_index = 0;
                                    expected_start_ms = 0.0;
                                    timeline = PlaybackTimeline::default();
                                    worker_position_ms.store(0, Ordering::Release);
                                    worker_queued_until_ms.store(0, Ordering::Release);
                                    worker_state.store(
                                        PlaybackSchedulerState::Stopped as u8,
                                        Ordering::Release,
                                    );
                                    continue;
                                }
                                match Sink::try_new(&worker_handle) {
                                    Ok(next) => {
                                        next.append(SamplesBuffer::new(
                                            channels.max(1),
                                            sample_rate,
                                            samples,
                                        ));
                                        next.set_volume(volume);
                                        next.set_speed(speed);
                                        next.play();
                                        sink = Some(next);
                                        final_chunk = is_final;
                                        expected_sample_rate = Some(sample_rate);
                                        expected_channels = Some(channels);
                                        active_generation = generation;
                                        expected_index = index.saturating_add(1);
                                        expected_start_ms = end_ms;
                                        timeline_end_ms = (end_ms - start_ms).max(0.0);
                                        timeline_start_ms = start_ms;
                                        worker_queued_until_ms.store(
                                            timeline_end_ms.round() as u64,
                                            Ordering::Release,
                                        );
                                        timeline_position_ms = 0.0;
                                        last_clock_tick = std::time::Instant::now();
                                        playback_speed = f64::from(speed.max(0.01));
                                        worker_position_ms.store(0, Ordering::Release);
                                        worker_state.store(
                                            PlaybackSchedulerState::Running as u8,
                                            Ordering::Release,
                                        );
                                        push_scheduler_event(
                                            &worker_events,
                                            PlaybackSchedulerEvent::PlaybackStarted {
                                                generation,
                                                start_ms,
                                            },
                                        );
                                        push_scheduler_event(
                                            &worker_events,
                                            PlaybackSchedulerEvent::ChunkAccepted {
                                                generation,
                                                index,
                                                start_ms,
                                                end_ms,
                                            },
                                        );
                                    }
                                    Err(error) => {
                                        crate::terminal_log::error(
                                            "Audio",
                                            format!("could not create sink: {error}"),
                                        );
                                        push_scheduler_event(
                                            &worker_events,
                                            PlaybackSchedulerEvent::ChunkRejected {
                                                generation,
                                                index,
                                                reason: format!(
                                                    "não foi possível abrir a saída de áudio: {error}"
                                                ),
                                            },
                                        );
                                        expected_sample_rate = None;
                                        expected_channels = None;
                                        active_generation = 0;
                                        expected_index = 0;
                                        expected_start_ms = 0.0;
                                        timeline = PlaybackTimeline::default();
                                        worker_position_ms.store(0, Ordering::Release);
                                        worker_queued_until_ms.store(0, Ordering::Release);
                                        worker_state.store(
                                            PlaybackSchedulerState::Stopped as u8,
                                            Ordering::Release,
                                        );
                                    }
                                }
                            }
                            SchedulerCommand::Append {
                                generation,
                                index,
                                start_ms,
                                end_ms,
                                samples,
                                sample_rate,
                                channels,
                                final_chunk: is_final,
                            } => {
                                if expected_sample_rate.is_none() {
                                    // Start was rejected (or could not open an
                                    // output sink). Keep its original error as
                                    // the diagnosis instead of flooding the UI
                                    // with misleading format mismatches.
                                    continue;
                                }
                                if generation != active_generation
                                    || index != expected_index
                                    || (start_ms - expected_start_ms).abs() > 0.25
                                    || !start_ms.is_finite()
                                    || !end_ms.is_finite()
                                    || end_ms <= start_ms
                                    || samples.len() % usize::from(channels.max(1)) != 0
                                    || ((samples.len() as f64
                                        / channels.max(1) as f64
                                        * 1000.0
                                        / sample_rate.max(1) as f64)
                                        - (end_ms - start_ms))
                                        .abs()
                                        > 1.0
                                    || expected_sample_rate != Some(sample_rate)
                                    || expected_channels != Some(channels)
                                    || sample_rate == 0
                                    || channels == 0
                                {
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::ChunkRejected {
                                            generation,
                                            index,
                                            reason: "geração, ordem, timeline ou formato incompatível"
                                                .to_string(),
                                        },
                                    );
                                    crate::terminal_log::warn(
                                        "Audio",
                                        format!(
                                        "rejected chunk: rate={sample_rate} channels={channels} expected_rate={:?} expected_channels={:?}",
                                        expected_sample_rate,
                                        expected_channels
                                    ));
                                    continue;
                                }
                                if let Some(active) = sink.as_ref() {
                                    if timeline.append(TimelineChunk {
                                        generation,
                                        index,
                                        start_ms,
                                        end_ms,
                                        is_final,
                                    }) != TimelineDecision::Accepted
                                    {
                                        push_scheduler_event(
                                            &worker_events,
                                            PlaybackSchedulerEvent::ChunkRejected {
                                                generation,
                                                index,
                                                reason: "timeline validator rejected append".to_string(),
                                            },
                                        );
                                        continue;
                                    }
                                    let was_waiting = matches!(
                                        PlaybackSchedulerState::from_raw(
                                            worker_state.load(Ordering::Acquire),
                                        ),
                                        PlaybackSchedulerState::Waiting
                                            | PlaybackSchedulerState::Underrun
                                    );
                                    active.append(SamplesBuffer::new(
                                        channels.max(1),
                                        sample_rate,
                                        samples,
                                    ));
                                    final_chunk = is_final;
                                    expected_index = expected_index.saturating_add(1);
                                    expected_start_ms = end_ms;
                                    timeline_end_ms = (end_ms - timeline_start_ms).max(timeline_end_ms);
                                    worker_queued_until_ms.store(
                                        timeline_end_ms.round() as u64,
                                        Ordering::Release,
                                    );
                                    // Do not count time spent waiting for a
                                    // renderer chunk. While already Running,
                                    // however, appending must not reset the
                                    // clock: doing that for every progressive
                                    // chunk makes the visual playhead lose the
                                    // command-delivery interval and slowly
                                    // fall behind the actual audio.
                                    if was_waiting {
                                        last_clock_tick = std::time::Instant::now();
                                    }
                                    worker_state.store(
                                        PlaybackSchedulerState::Running as u8,
                                        Ordering::Release,
                                    );
                                    if was_waiting {
                                        push_scheduler_event(
                                            &worker_events,
                                            PlaybackSchedulerEvent::PlaybackResumed {
                                                generation,
                                                played_ms: timeline_position_ms,
                                            },
                                        );
                                    }
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::ChunkAccepted {
                                            generation,
                                            index,
                                            start_ms,
                                            end_ms,
                                        },
                                    );
                                } else {
                                    crate::terminal_log::debug("Audio", "chunk received without active generation");
                                }
                            }
                            SchedulerCommand::SetSpeed(speed) => {
                                let speed = speed.clamp(0.25, 4.0);
                                playback_speed = f64::from(speed);
                                if let Some(active) = sink.as_ref() {
                                    active.set_speed(speed);
                                }
                                // Do not count the command-delivery interval
                                // twice when the transport rate changes.
                                last_clock_tick = std::time::Instant::now();
                            }
                            SchedulerCommand::Stop { generation } => {
                                if generation != worker_generation.load(Ordering::Acquire) {
                                    continue;
                                }
                                let played_ms = timeline_position_ms;
                                let had_active_playback = sink.is_some()
                                    || active_generation != 0
                                    || matches!(
                                        PlaybackSchedulerState::from_raw(
                                            worker_state.load(Ordering::Acquire),
                                        ),
                                        PlaybackSchedulerState::Running
                                            | PlaybackSchedulerState::Waiting
                                            | PlaybackSchedulerState::Underrun
                                    );
                                if let Some(active) = sink.take() {
                                    active.stop();
                                }
                                worker_position_ms.store(0, Ordering::Release);
                                worker_queued_until_ms.store(0, Ordering::Release);
                                timeline_position_ms = 0.0;
                                worker_state.store(
                                    PlaybackSchedulerState::Stopped as u8,
                                    Ordering::Release,
                                );
                                if had_active_playback {
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::PlaybackStopped {
                                            generation,
                                            played_ms,
                                        },
                                    );
                                }
                                final_chunk = false;
                                expected_sample_rate = None;
                                expected_channels = None;
                                active_generation = 0;
                                expected_index = 0;
                                expected_start_ms = 0.0;
                                timeline_end_ms = 0.0;
                                timeline_start_ms = 0.0;
                                timeline = PlaybackTimeline::default();
                            }
                            SchedulerCommand::ProducerClosed {
                                generation,
                                final_chunk_received,
                            } => {
                                // A bridge from an older render may close
                                // after a new generation has already started.
                                // Only the currently published generation is
                                // allowed to change the scheduler state.
                                if generation
                                    != worker_generation.load(Ordering::Acquire)
                                {
                                    continue;
                                }
                                if !final_chunk_received {
                                    if let Some(active) = sink.take() {
                                        active.stop();
                                    }
                                    timeline.stop();
                                    worker_state.store(
                                        PlaybackSchedulerState::Stopped as u8,
                                        Ordering::Release,
                                    );
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::ProducerClosed {
                                            generation,
                                            final_chunk_received,
                                            played_ms: timeline_position_ms,
                                        },
                                    );
                                    push_scheduler_event(
                                        &worker_events,
                                        PlaybackSchedulerEvent::PlaybackStopped {
                                            generation,
                                            played_ms: timeline_position_ms,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    if sink.is_some() {
                        if PlaybackSchedulerState::from_raw(
                            worker_state.load(Ordering::Acquire),
                        ) == PlaybackSchedulerState::Running
                        {
                            let now = std::time::Instant::now();
                            let elapsed_ms = now
                                .saturating_duration_since(last_clock_tick)
                                .as_secs_f64()
                                * 1000.0
                                * playback_speed;
                            timeline_position_ms += elapsed_ms;
                            last_clock_tick = now;
                            timeline_position_ms = timeline_position_ms.min(timeline_end_ms);
                            let timeline_decision = timeline.advance(elapsed_ms);
                            worker_position_ms.store(
                                timeline_position_ms.max(0.0).round() as u64,
                                Ordering::Release,
                            );
                            if matches!(
                                timeline_decision,
                                TimelineDecision::Finished | TimelineDecision::Waiting
                            ) {
                                let next_state = if final_chunk {
                                    PlaybackSchedulerState::Finished
                                } else {
                                    PlaybackSchedulerState::Waiting
                                };
                                worker_state.store(next_state as u8, Ordering::Release);
                                push_scheduler_event(
                                    &worker_events,
                                    if final_chunk {
                                        PlaybackSchedulerEvent::PlaybackFinished {
                                            generation: active_generation,
                                            played_ms: timeline_position_ms,
                                        }
                                    } else {
                                        PlaybackSchedulerEvent::WaitingForChunk {
                                            generation: active_generation,
                                            played_ms: timeline_position_ms,
                                        }
                                    },
                                );
                                last_clock_tick = now;
                            }
                        }
                    }
                }
            })
            .ok();

        Self {
            _stream: stream,
            stream_handle,
            active_sink: None,
            volume: 1.0,
            speed: 1.0,
            scheduler_tx: Some(scheduler_tx),
            scheduler_state,
            scheduler_position_ms,
            scheduler_queued_until_ms,
            playback_generation,
            scheduler_events,
        }
    }

    pub fn list_output_devices() -> Vec<String> {
        #[allow(unused_imports)]
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn default_host_name() -> String {
        #[allow(unused_imports)]
        use rodio::cpal::traits::HostTrait;
        let host = rodio::cpal::default_host();
        format!("{:?}", host.id())
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.play_samples_with_channels(samples, sample_rate, 1);
    }

    pub fn play_samples_with_channels(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        let _ = self.try_play_samples_with_channels(samples, sample_rate, channels);
    }

    pub fn try_play_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        if samples.is_empty() {
            return Err("O render não contém amostras de áudio.".to_string());
        }
        self.stop_scheduler();
        Self::sanitize_samples(&mut samples);

        if let Some(old_sink) = self.active_sink.take() {
            old_sink.pause();
            old_sink.detach();
        }

        let Some(handle) = self.stream_handle.as_ref() else {
            return Err("Nenhum dispositivo de saída de áudio está disponível.".to_string());
        };
        let sink = Sink::try_new(handle)
            .map_err(|error| format!("Não foi possível criar a saída de áudio: {error}"))?;
        let buffer = SamplesBuffer::new(channels.max(1), sample_rate, samples);
        sink.append(buffer);
        sink.set_volume(self.volume);
        sink.set_speed(self.speed);
        sink.play();
        self.active_sink = Some(sink);
        Ok(())
    }

    /// Queue the next contiguous preview chunk without interrupting playback.
    pub fn append_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_scheduler_samples(&mut samples);

        if self.active_sink.as_ref().is_some_and(|sink| sink.empty()) {
            // A sink can become empty between UI frames while the renderer
            // is still producing pages. Recreate it instead of appending to
            // an already-finished queue, which is ignored by some outputs.
            self.active_sink = None;
            self.play_samples_with_channels(samples, sample_rate, channels);
        } else if let Some(sink) = self.active_sink.as_ref() {
            sink.append(SamplesBuffer::new(channels.max(1), sample_rate, samples));
        } else {
            self.play_samples_with_channels(samples, sample_rate, channels);
        }
    }

    pub fn start_scheduler(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
    ) {
        if samples.is_empty() {
            return;
        }
        let mut samples = samples;
        // Progressive renderer output is authoritative.  The scheduler may
        // protect the device from NaN/Inf, but must not apply the legacy
        // preview soft-knee here: that would change the resampler's amplitude
        // before the wavtool/mixer and make chunks differ from the waveform.
        Self::sanitize_scheduler_samples(&mut samples);
        if let Some(tx) = &self.scheduler_tx {
            let _ = tx.send(SchedulerCommand::Start {
                generation: self.playback_generation.load(Ordering::Acquire),
                index: 0,
                start_ms: 0.0,
                end_ms: samples.len() as f64 * 1000.0
                    / (sample_rate.max(1) as f64 * channels.max(1) as f64),
                samples,
                sample_rate,
                channels,
                final_chunk,
                volume: self.volume,
                speed: self.speed,
            });
        }
    }

    pub fn append_scheduler_chunk(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        final_chunk: bool,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_scheduler_samples(&mut samples);
        if let Some(tx) = &self.scheduler_tx {
            let _ = tx.send(SchedulerCommand::Append {
                generation: self.playback_generation.load(Ordering::Acquire),
                index: 1,
                start_ms: 0.0,
                end_ms: samples.len() as f64 * 1000.0
                    / (sample_rate.max(1) as f64 * channels.max(1) as f64),
                samples,
                sample_rate,
                channels,
                final_chunk,
            });
        }
    }

    fn stop_scheduler(&mut self) {
        // Invalidate producers before stopping the sink. A renderer thread
        // may still be unwinding and must not be able to enqueue audio after
        // an explicit pause/stop or a new playback generation.
        self.playback_generation.fetch_add(1, Ordering::AcqRel);
        if let Some(tx) = &self.scheduler_tx {
            let _ = tx.send(SchedulerCommand::Stop {
                generation: self.playback_generation.load(Ordering::Acquire),
            });
        }
    }

    pub fn scheduler_is_active(&self) -> bool {
        matches!(
            self.scheduler_state(),
            PlaybackSchedulerState::Running
                | PlaybackSchedulerState::Waiting
                | PlaybackSchedulerState::Underrun
        )
    }

    pub fn scheduler_is_running(&self) -> bool {
        self.scheduler_state() == PlaybackSchedulerState::Running
    }

    pub fn scheduler_state(&self) -> PlaybackSchedulerState {
        PlaybackSchedulerState::from_raw(self.scheduler_state.load(Ordering::Acquire))
    }

    pub fn scheduler_position_ms(&self) -> f64 {
        self.scheduler_position_ms.load(Ordering::Acquire) as f64
    }

    pub fn scheduler_queued_until_ms(&self) -> f64 {
        self.scheduler_queued_until_ms.load(Ordering::Acquire) as f64
    }

    /// Position of the currently playing direct buffer, relative to its start.
    /// Rodio's position already accounts for Sink::set_speed().
    pub fn playback_position_ms(&self) -> Option<f64> {
        self.active_sink
            .as_ref()
            .map(|sink| sink.get_pos().as_secs_f64() * 1000.0)
    }

    pub fn drain_scheduler_events(&self) -> Vec<PlaybackSchedulerEvent> {
        let Ok(mut queue) = self.scheduler_events.lock() else {
            return Vec::new();
        };
        queue.drain(..).collect()
    }

    fn sanitize_samples(samples: &mut [f32]) {
        for s in samples.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            } else if FLUSH_DENORMALS_TO_ZERO.load(Ordering::Relaxed) && s.abs() < f32::MIN_POSITIVE
            {
                *s = 0.0;
            } else if s.abs() > 0.95 {
                let sign = s.signum();
                let excess = s.abs() - 0.90;
                let compressed = 0.90 + 0.08 * (excess / 0.08).tanh();
                *s = sign * compressed.min(0.98);
            }
        }
    }

    fn sanitize_scheduler_samples(samples: &mut [f32]) {
        for sample in samples {
            if !sample.is_finite() {
                *sample = 0.0;
            } else if FLUSH_DENORMALS_TO_ZERO.load(Ordering::Relaxed)
                && sample.abs() < f32::MIN_POSITIVE
            {
                *sample = 0.0;
            }
        }
    }

    pub fn stop(&mut self) {
        self.stop_scheduler();
        if let Some(sink) = self.active_sink.take() {
            sink.pause();
            sink.detach();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
        if let Some(sink) = &self.active_sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
        if let Some(sink) = &self.active_sink {
            sink.set_speed(self.speed);
        }
        if let Some(tx) = &self.scheduler_tx {
            let _ = tx.send(SchedulerCommand::SetSpeed(self.speed));
        }
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn is_playing(&self) -> bool {
        if self.scheduler_is_running() {
            return true;
        }
        if let Some(ref sink) = self.active_sink {
            return !sink.empty();
        }
        false
    }
}

#[cfg(target_arch = "wasm32")]
pub struct AudioPlayer {
    ctx: Option<web_sys::AudioContext>,
    scheduled_sources: Vec<web_sys::AudioBufferSourceNode>,
    next_play_time: f64,
    volume: f32,
    speed: f32,
    is_playing: bool,
    playback_start_time: f64,
}

#[cfg(target_arch = "wasm32")]
impl AudioPlayer {
    pub fn begin_progressive_generation(&mut self) -> u64 {
        0
    }

    pub fn progressive_handle(&self) -> Option<()> {
        None
    }

    pub fn scheduler_is_running(&self) -> bool {
        false
    }

    pub fn scheduler_is_active(&self) -> bool {
        self.is_playing
    }

    pub fn set_flush_denormals_to_zero(&mut self, enabled: bool) {
        set_flush_denormals_to_zero(enabled);
    }

    pub fn new() -> Self {
        Self {
            ctx: None,
            scheduled_sources: Vec::new(),
            next_play_time: 0.0,
            volume: 1.0,
            speed: 1.0,
            is_playing: false,
            playback_start_time: 0.0,
        }
    }

    /// Web Audio must be resumed from the user gesture that requested
    /// playback. Calling this before rendering prevents autoplay policies
    /// from rejecting the first scheduled buffer after an async render.
    pub fn prepare_for_playback(&mut self) -> Result<(), String> {
        self.get_or_create_context().map(|_| ()).ok_or_else(|| {
            "O navegador não conseguiu inicializar o dispositivo de áudio Web Audio.".to_string()
        })
    }

    fn get_or_create_context(&mut self) -> Option<&web_sys::AudioContext> {
        if self.ctx.is_none() {
            if let Ok(ctx) = web_sys::AudioContext::new() {
                self.ctx = Some(ctx);
            }
        }
        if let Some(ref ctx) = self.ctx {
            let _ = ctx.resume();
        }
        self.ctx.as_ref()
    }

    pub fn list_output_devices() -> Vec<String> {
        vec!["Padrão do Navegador (Web Audio)".to_string()]
    }

    pub fn default_host_name() -> String {
        "Web Audio API".to_string()
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.play_samples_with_channels(samples, sample_rate, 1);
    }

    pub fn play_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_samples(&mut samples);
        self.stop();

        let volume = self.volume;
        let speed = self.speed;

        if let Some(ctx) = self.get_or_create_context() {
            let num_channels = channels.max(1) as u32;
            let frame_count = (samples.len() / num_channels as usize) as u32;
            if frame_count == 0 {
                return;
            }
            if let Ok(buffer) = ctx.create_buffer(num_channels, frame_count, sample_rate as f32) {
                for ch in 0..num_channels {
                    let mut ch_data = Vec::with_capacity(frame_count as usize);
                    for i in 0..frame_count as usize {
                        let sample_idx = i * num_channels as usize + ch as usize;
                        if sample_idx < samples.len() {
                            ch_data.push(samples[sample_idx] * volume);
                        }
                    }
                    let _ = buffer.copy_to_channel(&ch_data, ch as i32);
                }
                if let Ok(source) = ctx.create_buffer_source() {
                    let _ = source.set_buffer(Some(&buffer));
                    source.playback_rate().set_value(speed);
                    let _ = source.connect_with_audio_node(&ctx.destination());
                    let now = ctx.current_time();
                    let dur = (frame_count as f64) / (sample_rate as f64);
                    let _ = source.start_with_when(now);
                    self.playback_start_time = now;
                    self.next_play_time = now + dur / speed.max(0.001) as f64;
                    self.scheduled_sources.push(source);
                    self.is_playing = true;
                }
            }
        }
    }

    pub fn try_play_samples_with_channels(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        self.play_samples_with_channels(samples, sample_rate, channels);
        if self.is_playing {
            Ok(())
        } else {
            Err("O navegador não conseguiu iniciar a reprodução Web Audio.".to_string())
        }
    }

    pub fn append_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        if !self.is_playing || self.scheduled_sources.is_empty() {
            self.play_samples_with_channels(samples, sample_rate, channels);
            return;
        }
        Self::sanitize_samples(&mut samples);

        let volume = self.volume;
        let speed = self.speed;

        if let Some(ctx) = self.get_or_create_context() {
            let num_channels = channels.max(1) as u32;
            let frame_count = (samples.len() / num_channels as usize) as u32;
            if frame_count == 0 {
                return;
            }
            if let Ok(buffer) = ctx.create_buffer(num_channels, frame_count, sample_rate as f32) {
                for ch in 0..num_channels {
                    let mut ch_data = Vec::with_capacity(frame_count as usize);
                    for i in 0..frame_count as usize {
                        let sample_idx = i * num_channels as usize + ch as usize;
                        if sample_idx < samples.len() {
                            ch_data.push(samples[sample_idx] * volume);
                        }
                    }
                    let _ = buffer.copy_to_channel(&ch_data, ch as i32);
                }
                if let Ok(source) = ctx.create_buffer_source() {
                    let _ = source.set_buffer(Some(&buffer));
                    source.playback_rate().set_value(speed);
                    let _ = source.connect_with_audio_node(&ctx.destination());
                    let now = ctx.current_time();
                    let start_time = self.next_play_time.max(now);
                    let dur = (frame_count as f64) / (sample_rate as f64);
                    let _ = source.start_with_when(start_time);
                    self.next_play_time = start_time + dur;
                    self.scheduled_sources.push(source);
                }
            }
        }
    }

    fn sanitize_samples(samples: &mut [f32]) {
        for s in samples.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            } else if FLUSH_DENORMALS_TO_ZERO.load(Ordering::Relaxed) && s.abs() < f32::MIN_POSITIVE
            {
                *s = 0.0;
            } else if s.abs() > 0.95 {
                let sign = s.signum();
                let excess = s.abs() - 0.90;
                let compressed = 0.90 + 0.08 * (excess / 0.08).tanh();
                *s = sign * compressed.min(0.98);
            }
        }
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.next_play_time = 0.0;
        for src in self.scheduled_sources.drain(..) {
            let _ = src.stop_with_when(0.0);
            let _ = src.disconnect();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
    }

    pub fn playback_position_ms(&self) -> Option<f64> {
        if !self.is_playing {
            return None;
        }
        self.ctx.as_ref().map(|ctx| {
            (ctx.current_time() - self.playback_start_time).max(0.0) * self.speed as f64 * 1000.0
        })
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn is_playing(&self) -> bool {
        if !self.is_playing {
            return false;
        }
        if let Some(ref ctx) = self.ctx {
            ctx.current_time() < self.next_play_time
        } else {
            false
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl AudioPlayer {
    pub fn start_scheduler(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        _final_chunk: bool,
    ) {
        self.play_samples_with_channels(samples, sample_rate, channels);
    }

    pub fn append_scheduler_chunk(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        _final_chunk: bool,
    ) {
        self.append_samples_with_channels(samples, sample_rate, channels);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AudioPlayer {
    /// Desktop output is initialized by `new`; this keeps the playback call
    /// site identical across native and WebAssembly targets.
    pub fn prepare_for_playback(&mut self) -> Result<(), String> {
        let Some(handle) = self.stream_handle.as_ref() else {
            return Err("Nenhum dispositivo de saída de áudio está disponível.".to_string());
        };

        let _ = handle;
        Ok(())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_play_samples() {
        let mut player = AudioPlayer::new();
        let samples = vec![0.1f32; 4410];
        player.play_samples(samples, 44100);
        player.stop();
        assert!(!player.is_playing());
    }

    #[test]
    fn progressive_scheduler_preserves_finite_amplitude() {
        let mut samples = vec![0.97_f32, -0.99_f32, f32::NAN, f32::INFINITY];
        AudioPlayer::sanitize_scheduler_samples(&mut samples);
        assert_eq!(samples[0], 0.97);
        assert_eq!(samples[1], -0.99);
        assert_eq!(samples[2], 0.0);
        assert_eq!(samples[3], 0.0);
    }

    #[test]
    fn stopping_an_inactive_scheduler_does_not_emit_a_transport_stop() {
        let mut player = AudioPlayer::new();
        let _ = player.drain_scheduler_events();
        player.stop_scheduler();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(player.drain_scheduler_events().is_empty());
    }

    #[test]
    fn direct_sink_reports_an_advancing_playback_position_when_audio_is_available() {
        let mut player = AudioPlayer::new();
        if player.prepare_for_playback().is_err() {
            return;
        }
        let samples = vec![0.0_f32; 44_100];
        player
            .try_play_samples_with_channels(samples, 44_100, 1)
            .expect("the initialized output device should accept a direct buffer");
        std::thread::sleep(std::time::Duration::from_millis(80));
        let position_ms = player
            .playback_position_ms()
            .expect("a direct sink should publish its playback position");
        player.stop();
        assert!(
            position_ms > 0.0,
            "direct sink position remained frozen at {position_ms}ms"
        );
    }
}
