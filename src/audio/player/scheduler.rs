use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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
#[derive(Clone)]
pub struct ProgressivePlaybackHandle {
    pub(super) tx: Sender<SchedulerCommand>,
    pub(super) volume: f32,
    pub(super) speed: f32,
    pub(super) generation: u64,
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
    pub(super) fn from_raw(value: u8) -> Self {
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
pub(super) fn push_scheduler_event(
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
pub(super) enum SchedulerCommand {
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
