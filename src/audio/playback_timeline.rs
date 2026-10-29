#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineState {
    Idle,
    Running,
    Waiting,
    Underrun,
    Finished,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineChunk {
    pub generation: u64,
    pub index: usize,
    pub start_ms: f64,
    pub end_ms: f64,
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimelineDecision {
    Accepted,
    Rejected(&'static str),
    Waiting,
    Underrun,
    Finished,
}

#[derive(Debug, Clone)]
pub struct PlaybackTimeline {
    generation: u64,
    next_index: usize,
    next_start_ms: f64,
    queued_until_ms: f64,
    base_start_ms: f64,
    played_until_ms: f64,
    final_received: bool,
    state: TimelineState,
}

impl Default for PlaybackTimeline {
    fn default() -> Self {
        Self {
            generation: 0,
            next_index: 0,
            next_start_ms: 0.0,
            queued_until_ms: 0.0,
            base_start_ms: 0.0,
            played_until_ms: 0.0,
            final_received: false,
            state: TimelineState::Idle,
        }
    }
}

impl PlaybackTimeline {
    pub fn start(&mut self, chunk: TimelineChunk) -> TimelineDecision {
        if !chunk.start_ms.is_finite()
            || !chunk.end_ms.is_finite()
            || chunk.end_ms <= chunk.start_ms
        {
            return TimelineDecision::Rejected("invalid interval");
        }
        self.generation = chunk.generation;
        self.next_index = chunk.index.saturating_add(1);
        self.next_start_ms = chunk.end_ms;
        self.queued_until_ms = chunk.end_ms - chunk.start_ms;
        self.base_start_ms = chunk.start_ms;
        self.played_until_ms = 0.0;
        self.final_received = chunk.is_final;
        self.state = TimelineState::Running;
        TimelineDecision::Accepted
    }

    pub fn append(&mut self, chunk: TimelineChunk) -> TimelineDecision {
        if self.final_received {
            return TimelineDecision::Rejected("after final chunk");
        }
        if chunk.generation != self.generation {
            return TimelineDecision::Rejected("old generation");
        }
        if chunk.index != self.next_index {
            return TimelineDecision::Rejected("out of order");
        }
        if (chunk.start_ms - self.next_start_ms).abs() > 0.25 {
            return TimelineDecision::Rejected("timeline gap");
        }
        if !chunk.end_ms.is_finite() || chunk.end_ms <= chunk.start_ms {
            return TimelineDecision::Rejected("invalid interval");
        }
        self.next_index = chunk.index.saturating_add(1);
        self.next_start_ms = chunk.end_ms;
        self.queued_until_ms = chunk.end_ms - self.base_start_ms;
        self.final_received = chunk.is_final;
        if self.state == TimelineState::Waiting || self.state == TimelineState::Underrun {
            self.state = TimelineState::Running;
        }
        TimelineDecision::Accepted
    }

    pub fn advance(&mut self, elapsed_ms: f64) -> TimelineDecision {
        if self.state != TimelineState::Running || !elapsed_ms.is_finite() {
            return TimelineDecision::Rejected("timeline is not running");
        }
        self.played_until_ms =
            (self.played_until_ms + elapsed_ms.max(0.0)).min(self.queued_until_ms);
        if self.played_until_ms + 5.0 >= self.queued_until_ms {
            self.state = if self.final_received {
                TimelineState::Finished
            } else {
                TimelineState::Waiting
            };
            return if self.final_received {
                TimelineDecision::Finished
            } else {
                TimelineDecision::Waiting
            };
        }
        TimelineDecision::Accepted
    }

    pub fn report_underrun(&mut self) -> TimelineDecision {
        if self.state == TimelineState::Running {
            self.state = TimelineState::Underrun;
            TimelineDecision::Underrun
        } else {
            TimelineDecision::Rejected("timeline is not running")
        }
    }

    /// Marks the end of a producer's available audio. If playback has already
    /// reached that boundary, finish now; otherwise `advance` will finish it
    /// after the queued audio drains.
    pub fn finish(&mut self) -> TimelineDecision {
        self.final_received = true;
        if matches!(self.state, TimelineState::Waiting | TimelineState::Underrun)
            && self.played_until_ms + 5.0 >= self.queued_until_ms
        {
            self.state = TimelineState::Finished;
            TimelineDecision::Finished
        } else {
            TimelineDecision::Accepted
        }
    }

    pub fn stop(&mut self) {
        self.state = TimelineState::Stopped;
    }

    pub fn state(&self) -> TimelineState {
        self.state
    }

    pub fn played_until_ms(&self) -> f64 {
        self.played_until_ms
    }

    pub fn queued_until_ms(&self) -> f64 {
        self.queued_until_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(
        generation: u64,
        index: usize,
        start_ms: f64,
        end_ms: f64,
        is_final: bool,
    ) -> TimelineChunk {
        TimelineChunk {
            generation,
            index,
            start_ms,
            end_ms,
            is_final,
        }
    }

    #[test]
    fn ui_delay_does_not_change_the_audio_timeline() {
        let mut timeline = PlaybackTimeline::default();
        assert_eq!(
            timeline.start(chunk(1, 0, 0.0, 1000.0, false)),
            TimelineDecision::Accepted
        );
        assert_eq!(timeline.advance(500.0), TimelineDecision::Accepted);
        assert_eq!(timeline.played_until_ms(), 500.0);
        assert_eq!(
            timeline.append(chunk(1, 1, 1000.0, 2000.0, true)),
            TimelineDecision::Accepted
        );
        assert_eq!(timeline.queued_until_ms(), 2000.0);
        assert_eq!(timeline.advance(500.0), TimelineDecision::Accepted);
        assert_eq!(timeline.advance(1000.0), TimelineDecision::Finished);
        assert_eq!(timeline.played_until_ms(), 2000.0);
    }

    #[test]
    fn gap_and_old_generation_are_rejected_without_mutating_queue() {
        let mut timeline = PlaybackTimeline::default();
        timeline.start(chunk(7, 0, 0.0, 1000.0, false));
        assert_eq!(
            timeline.append(chunk(6, 1, 1000.0, 2000.0, false)),
            TimelineDecision::Rejected("old generation")
        );
        assert_eq!(
            timeline.append(chunk(7, 2, 1000.0, 3000.0, false)),
            TimelineDecision::Rejected("out of order")
        );
        assert_eq!(timeline.queued_until_ms(), 1000.0);
    }

    #[test]
    fn underrun_freezes_and_resumes_without_restarting_chunk() {
        let mut timeline = PlaybackTimeline::default();
        timeline.start(chunk(2, 0, 0.0, 1000.0, false));
        timeline.advance(400.0);
        assert_eq!(timeline.report_underrun(), TimelineDecision::Underrun);
        assert_eq!(timeline.played_until_ms(), 400.0);
        assert_eq!(
            timeline.append(chunk(2, 1, 1000.0, 2000.0, true)),
            TimelineDecision::Accepted
        );
        assert_eq!(timeline.played_until_ms(), 400.0);
        assert_eq!(timeline.advance(1600.0), TimelineDecision::Finished);
    }

    #[test]
    fn duplicate_and_out_of_order_chunks_do_not_advance_timeline() {
        let mut timeline = PlaybackTimeline::default();
        timeline.start(chunk(3, 0, 0.0, 1000.0, false));
        assert_eq!(
            timeline.append(chunk(3, 0, 0.0, 1000.0, false)),
            TimelineDecision::Rejected("out of order")
        );
        assert_eq!(
            timeline.append(chunk(3, 2, 1000.0, 2000.0, true)),
            TimelineDecision::Rejected("out of order")
        );
        assert_eq!(timeline.queued_until_ms(), 1000.0);
    }

    #[test]
    fn a_final_chunk_closes_the_generation() {
        let mut timeline = PlaybackTimeline::default();
        timeline.start(chunk(4, 0, 0.0, 1000.0, true));
        assert_eq!(
            timeline.append(chunk(4, 1, 1000.0, 2000.0, false)),
            TimelineDecision::Rejected("after final chunk")
        );
        assert_eq!(timeline.advance(1000.0), TimelineDecision::Finished);
    }
}
