//! Structured render progress, independent from human-readable diagnostics.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStage {
    Preparing,
    Rendering,
    Mixing,
    Finalizing,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderProgressEvent {
    pub stage: RenderStage,
    /// Fraction within the current stage. `None` means the stage has no
    /// reliable denominator and the UI should show indeterminate activity.
    pub fraction: Option<f32>,
    pub completed: Option<usize>,
    pub total: Option<usize>,
}

/// Aggregates per-track phone progress into one monotonic synthesis stage.
pub(crate) struct TrackProgressAggregator {
    weights: Vec<usize>,
    values: std::sync::Mutex<Vec<f32>>,
    total_work: usize,
    active_tracks: usize,
}

impl TrackProgressAggregator {
    pub(crate) fn new(weights: Vec<usize>) -> Self {
        let total_work = weights.iter().sum::<usize>().max(1);
        let active_tracks = weights.iter().filter(|weight| **weight > 0).count();
        let values = vec![0.0; weights.len()];
        Self {
            weights,
            values: std::sync::Mutex::new(values),
            total_work,
            active_tracks,
        }
    }

    pub(crate) fn update(&self, track_index: usize, fraction: f32) -> Option<RenderProgressEvent> {
        let mut values = self.values.lock().ok()?;
        let current = *values.get(track_index)?;
        *values.get_mut(track_index)? = current.max(fraction.clamp(0.0, 1.0));
        let weighted = values
            .iter()
            .zip(&self.weights)
            .map(|(value, weight)| *value * *weight as f32)
            .sum::<f32>()
            / self.total_work as f32;
        let completed = values
            .iter()
            .zip(&self.weights)
            .filter(|(value, weight)| **weight > 0 && **value >= 1.0)
            .count();
        Some(RenderProgressEvent::with_fraction(
            RenderStage::Rendering,
            weighted,
            completed,
            self.active_tracks,
        ))
    }
}

impl RenderProgressEvent {
    pub fn stage(stage: RenderStage) -> Self {
        Self {
            stage,
            fraction: None,
            completed: None,
            total: None,
        }
    }

    pub fn counted(stage: RenderStage, completed: usize, total: usize) -> Self {
        let total = total.max(1);
        Self::with_fraction(
            stage,
            (completed as f32 / total as f32).clamp(0.0, 1.0),
            completed.min(total),
            total,
        )
    }

    pub fn with_fraction(
        stage: RenderStage,
        fraction: f32,
        completed: usize,
        total: usize,
    ) -> Self {
        let total = total.max(1);
        Self {
            stage,
            fraction: Some(fraction.clamp(0.0, 1.0)),
            completed: Some(completed.min(total)),
            total: Some(total),
        }
    }

    pub fn units(stage: RenderStage, completed: usize, total: usize) -> Self {
        let total = total.max(1);
        Self {
            stage,
            fraction: None,
            completed: Some(completed.min(total)),
            total: Some(total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderProgressEvent, RenderStage, TrackProgressAggregator};

    #[test]
    fn track_progress_aggregation_is_monotonic_when_callbacks_arrive_out_of_order() {
        let aggregate = TrackProgressAggregator::new(vec![5, 2]);
        let first = aggregate.update(0, 0.7).unwrap();
        let second = aggregate.update(1, 0.5).unwrap();
        let late = aggregate.update(0, 0.2).unwrap();

        assert_eq!(first.fraction, Some(0.5));
        assert_eq!(second.fraction, Some(4.5 / 7.0));
        assert_eq!(late.fraction, second.fraction);
        assert_eq!((late.completed, late.total), (Some(0), Some(2)));
        assert_eq!(aggregate.update(1, 1.0).unwrap().completed, Some(1));
    }

    #[test]
    fn progress_events_clamp_counts_and_preserve_indeterminate_stages() {
        let counted = RenderProgressEvent::counted(RenderStage::Mixing, 8, 4);
        assert_eq!(counted.fraction, Some(1.0));
        assert_eq!((counted.completed, counted.total), (Some(4), Some(4)));
        let stage = RenderProgressEvent::stage(RenderStage::Preparing);
        assert_eq!(stage.fraction, None);
        assert_eq!((stage.completed, stage.total), (None, None));
    }
}
